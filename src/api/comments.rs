use dioxus::prelude::*;
use uuid::Uuid;

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Comment {
    pub id: i64,
    pub author_id: Option<Uuid>,
    pub author: Option<String>,
    pub body: Option<String>,
    pub version_number: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub has_replies: bool,
}

#[server]
pub async fn thread(
    recipe: Uuid,
    parent: Option<i64>,
    after: i64,
) -> Result<Vec<Comment>, ServerFnError> {
    crate::security::enforce_request_budget(None, crate::security::RequestClass::PublicDiscovery)
        .map_err(|_| ServerFnError::new("Please try again later"))?;
    read_thread(&crate::db::get_pool(), recipe, parent, after).await
}

#[cfg(feature = "server")]
async fn read_thread(
    pool: &sqlx::PgPool,
    recipe: Uuid,
    parent: Option<i64>,
    after: i64,
) -> Result<Vec<Comment>, ServerFnError> {
    sqlx::query_as::<_, Comment>("SELECT c.id,c.author_id,u.username AS author,c.body,c.version_number,c.created_at,
        EXISTS(SELECT 1 FROM recipe_comments child WHERE child.recipe_id=c.recipe_id AND child.parent_id=c.id) AS has_replies
        FROM recipe_comments c JOIN recipes r ON r.id=c.recipe_id LEFT JOIN users u ON u.id=c.author_id
        WHERE c.recipe_id=$1 AND r.visibility='public' AND c.parent_id IS NOT DISTINCT FROM $2 AND c.id>$3 ORDER BY c.id LIMIT 21")
        .bind(recipe).bind(parent).bind(after).fetch_all(pool).await.map_err(|_| ServerFnError::new("Could not load comments"))
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub enum Change {
    Publish {
        parent: Option<i64>,
        body: String,
        request_id: Uuid,
    },
    Edit {
        id: i64,
        body: String,
    },
    Report {
        id: i64,
        observed_body: String,
        reason: String,
    },
    Delete {
        id: i64,
    },
}

#[server]
pub async fn change(
    expected: Option<Uuid>,
    recipe: Uuid,
    action: Change,
) -> Result<(), ServerFnError> {
    let user = crate::auth::account_binding::require_form_account(expected).await?;
    crate::security::enforce_user_request_budget(user, crate::security::RequestClass::Mutation)
        .map_err(|_| ServerFnError::new("Please try again later"))?;
    apply(&crate::db::get_pool(), user, recipe, action).await
}

#[cfg(feature = "server")]
async fn apply(
    pool: &sqlx::PgPool,
    user: Uuid,
    recipe: Uuid,
    action: Change,
) -> Result<(), ServerFnError> {
    let failure = |_| ServerFnError::new("Comment could not be saved");
    let mut tx = pool.begin().await.map_err(failure)?;
    // Serialize visibility, publication and deletion with changes to this thread.
    sqlx::query("SELECT id FROM recipes WHERE id=$1 AND visibility='public' FOR UPDATE")
        .bind(recipe)
        .fetch_one(&mut *tx)
        .await
        .map_err(failure)?;
    let validate = |body: String| {
        let body = body.trim().to_owned();
        if body.is_empty() || body.chars().count() > 2000 {
            Err(ServerFnError::new("Use 1 to 2000 characters"))
        } else {
            Ok(body)
        }
    };
    match action {
        Change::Publish {
            parent,
            body,
            request_id,
        } => {
            sqlx::query(
                "INSERT INTO recipe_comments(recipe_id,version_number,parent_id,author_id,body,request_id)
                SELECT $1,max(version_number),$2,$3,$4,$5 FROM recipe_versions WHERE recipe_id=$1 ON CONFLICT (author_id,request_id) DO NOTHING",
            )
            .bind(recipe)
            .bind(parent)
            .bind(user)
            .bind(validate(body)?)
            .bind(request_id)
            .execute(&mut *tx)
            .await
            .map_err(failure)?;
        }
        Change::Edit { id, body } => {
            sqlx::query("UPDATE recipe_comments SET body=$4 WHERE recipe_id=$1 AND id=$2 AND author_id=$3 RETURNING id")
                .bind(recipe).bind(id).bind(user).bind(validate(body)?).fetch_one(&mut *tx).await.map_err(failure)?;
        }
        Change::Report {
            id,
            observed_body,
            reason,
        } => {
            let reason = reason.trim();
            if reason.is_empty() || reason.chars().count() > 500 {
                return Err(ServerFnError::new(
                    "Use 1 to 500 characters for your report",
                ));
            }
            let threshold: i64 = std::env::var("COMMENT_REPORT_THRESHOLD")
                .ok()
                .and_then(|value| value.parse().ok())
                .filter(|n| *n > 0)
                .unwrap_or(3);
            // The recipe lock above serializes submissions with edits, deletes and other reports.
            let version: i32 = sqlx::query_scalar(
                "SELECT version_number FROM recipe_comments
                 WHERE recipe_id=$1 AND id=$2 AND body=$3",
            )
            .bind(recipe)
            .bind(id)
            .bind(&observed_body)
            .fetch_optional(&mut *tx)
            .await
            .map_err(failure)?
            .ok_or_else(|| ServerFnError::new("Comment changed or unavailable"))?;
            let case_id: i64 = sqlx::query_scalar(
                "INSERT INTO comment_review_cases (recipe_id,comment_id) VALUES ($1,$2)
                 ON CONFLICT (comment_id) WHERE resolved_at IS NULL
                 DO UPDATE SET comment_id=EXCLUDED.comment_id RETURNING id",
            )
            .bind(recipe)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(failure)?;
            sqlx::query("INSERT INTO comment_reports (case_id,reporter_id,reason,reported_body,recipe_version)
                         VALUES ($1,$2,$3,$4,$5) ON CONFLICT (case_id,reporter_id) DO NOTHING")
                .bind(case_id).bind(user).bind(reason).bind(observed_body).bind(version)
                .execute(&mut *tx).await.map_err(failure)?;
            sqlx::query(
                "UPDATE comment_review_cases SET queued_at=now(), status='queued'
                         WHERE id=$1 AND queued_at IS NULL
                         AND (SELECT count(*) FROM comment_reports WHERE case_id=$1) >= $2",
            )
            .bind(case_id)
            .bind(threshold)
            .execute(&mut *tx)
            .await
            .map_err(failure)?;
        }
        Change::Delete { id } => {
            sqlx::query("UPDATE recipe_comments SET body=NULL,author_id=NULL WHERE recipe_id=$1 AND id=$2 AND author_id=$3 RETURNING id")
                .bind(recipe).bind(id).bind(user).fetch_one(&mut *tx).await.map_err(failure)?;
            let mut candidate = Some(id);
            while let Some(id) = candidate {
                candidate = sqlx::query_scalar::<_, Option<i64>>("DELETE FROM recipe_comments c WHERE c.recipe_id=$1 AND c.id=$2 AND c.body IS NULL
                    AND NOT EXISTS(SELECT 1 FROM recipe_comments child WHERE child.recipe_id=$1 AND child.parent_id=c.id) RETURNING parent_id")
                    .bind(recipe).bind(id).fetch_optional(&mut *tx).await.map_err(failure)?.flatten();
            }
        }
    }
    tx.commit().await.map_err(failure)
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[tokio::test]
    async fn recipe_discussion_reports_queue_once_and_preserve_evidence() {
        let (_db, pool) = crate::test_utils::setup_test_db().await;
        let users: Vec<Uuid> = sqlx::query_scalar(
            "INSERT INTO users(username,display_name,email)
             SELECT 'cook'||n,'Cook '||n,n||'@example.test' FROM generate_series(1,3) n RETURNING id")
            .fetch_all(&pool).await.unwrap();
        let recipe: Uuid = sqlx::query_scalar(
            "INSERT INTO recipes(user_id,title,visibility) VALUES ($1,'Soup','public') RETURNING id")
            .bind(users[0]).fetch_one(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO recipe_versions(recipe_id,version_number,snapshot) VALUES ($1,1,'{}')",
        )
        .bind(recipe)
        .execute(&pool)
        .await
        .unwrap();
        let publish = |parent| Change::Publish {
            parent,
            body: "Original".into(),
            request_id: Uuid::new_v4(),
        };
        apply(&pool, users[0], recipe, publish(None)).await.unwrap();
        let root = read_thread(&pool, recipe, None, 0).await.unwrap().remove(0);
        apply(&pool, users[0], recipe, publish(Some(root.id)))
            .await
            .unwrap();
        let reply = read_thread(&pool, recipe, Some(root.id), 0)
            .await
            .unwrap()
            .remove(0);
        let report = |user, reason: &str, body: &str| {
            apply(
                &pool,
                user,
                recipe,
                Change::Report {
                    id: reply.id,
                    observed_body: body.to_owned(),
                    reason: reason.to_owned(),
                },
            )
        };
        let counts = || {
            sqlx::query_as::<_,(i64,i64)>(
            "SELECT (SELECT count(*) FROM comment_reports), count(*) FROM comment_review_cases WHERE queued_at IS NOT NULL").fetch_one(&pool)
        };
        assert!(report(users[0], "  ", "Original").await.is_err());
        assert!(report(users[0], "Spam", "Stale").await.is_err());
        report(users[0], "Spam", "Original").await.unwrap();
        report(users[0], "Retry", "Original").await.unwrap();
        report(users[1], "Spam", "Original").await.unwrap();
        let status = || {
            sqlx::query_scalar::<_, String>(
                "SELECT status::text FROM comment_review_cases WHERE resolved_at IS NULL",
            )
            .fetch_one(&pool)
        };
        assert_eq!(status().await.unwrap(), "collecting");
        assert_eq!(counts().await.unwrap(), (2, 0));
        tokio::try_join!(
            report(users[2], "Spam", "Original"),
            report(users[2], "Retry", "Original")
        )
        .unwrap();
        assert_eq!(status().await.unwrap(), "queued");
        assert_eq!(counts().await.unwrap(), (3, 1));
        assert_eq!(
            read_thread(&pool, recipe, Some(root.id), 0).await.unwrap()[0].body,
            reply.body
        );
        sqlx::query("UPDATE comment_review_cases SET resolved_at=now()")
            .execute(&pool)
            .await
            .unwrap();
        report(users[0], "New cycle", "Original").await.unwrap();
        assert_eq!(counts().await.unwrap(), (4, 1));
        sqlx::query("UPDATE recipes SET visibility='private' WHERE id=$1")
            .bind(recipe)
            .execute(&pool)
            .await
            .unwrap();
        assert!(report(users[1], "Private", "Original").await.is_err());
        sqlx::query("UPDATE recipes SET visibility='public' WHERE id=$1")
            .bind(recipe)
            .execute(&pool)
            .await
            .unwrap();
        apply(&pool, users[0], recipe, Change::Delete { id: reply.id })
            .await
            .unwrap();
        assert!(report(users[1], "Deleted", "Original").await.is_err());
        let retained: i64 = sqlx::query_scalar("SELECT count(*) FROM comment_reports WHERE reported_body='Original' AND recipe_version=1")
            .fetch_one(&pool).await.unwrap();
        assert_eq!(retained, 4);
    }

    #[tokio::test]
    async fn nested_recipe_discussion_lifecycle() {
        let (_db, pool) = crate::test_utils::setup_test_db().await;
        let user: Uuid = sqlx::query_scalar("INSERT INTO users(username,display_name,email) VALUES ('cook','Cook','cook@example.test') RETURNING id").fetch_one(&pool).await.unwrap();
        let recipe: Uuid = sqlx::query_scalar(
            "INSERT INTO recipes(user_id,title,visibility) VALUES($1,'Soup','public') RETURNING id",
        )
        .bind(user)
        .fetch_one(&pool)
        .await
        .unwrap();
        let version =
            "INSERT INTO recipe_versions(recipe_id,version_number,snapshot) VALUES($1,$2,'{}')";
        sqlx::query(version)
            .bind(recipe)
            .bind(2)
            .execute(&pool)
            .await
            .unwrap();
        let publish = |parent| Change::Publish {
            request_id: Uuid::new_v4(),
            parent,
            body: "Cooking note".into(),
        };
        apply(&pool, user, recipe, publish(None)).await.unwrap();
        let root = read_thread(&pool, recipe, None, 0).await.unwrap().remove(0);
        assert_eq!(root.version_number, 2);
        sqlx::query(version)
            .bind(recipe)
            .bind(3)
            .execute(&pool)
            .await
            .unwrap();
        apply(&pool, user, recipe, publish(Some(root.id)))
            .await
            .unwrap();
        let reply = read_thread(&pool, recipe, Some(root.id), 0)
            .await
            .unwrap()
            .remove(0);
        assert_eq!(reply.version_number, 3);
        apply(&pool, user, recipe, publish(Some(reply.id)))
            .await
            .unwrap();
        assert!(apply(
            &pool,
            Uuid::new_v4(),
            recipe,
            Change::Delete { id: root.id }
        )
        .await
        .is_err());
        apply(
            &pool,
            user,
            recipe,
            Change::Edit {
                id: root.id,
                body: "New note".into(),
            },
        )
        .await
        .unwrap();
        assert_eq!(
            read_thread(&pool, recipe, None, 0).await.unwrap()[0].version_number,
            2
        );
        apply(&pool, user, recipe, Change::Delete { id: root.id })
            .await
            .unwrap();
        let deleted = read_thread(&pool, recipe, None, 0).await.unwrap().remove(0);
        assert!(deleted.body.is_none() && deleted.author_id.is_none() && deleted.has_replies);
        let leaf = read_thread(&pool, recipe, Some(reply.id), 0)
            .await
            .unwrap()
            .remove(0);
        apply(&pool, user, recipe, Change::Delete { id: leaf.id })
            .await
            .unwrap();
        assert!(read_thread(&pool, recipe, Some(reply.id), 0)
            .await
            .unwrap()
            .is_empty());
        sqlx::query("UPDATE recipes SET visibility='private' WHERE id=$1")
            .bind(recipe)
            .execute(&pool)
            .await
            .unwrap();
        assert!(read_thread(&pool, recipe, Some(root.id), 0)
            .await
            .unwrap()
            .is_empty());
        assert!(apply(&pool, user, recipe, publish(None)).await.is_err());
    }
}
