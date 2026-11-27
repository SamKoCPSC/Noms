use dioxus::prelude::*;
use uuid::Uuid;

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Comment {
    pub id: i64,
    pub author_id: Option<Uuid>,
    pub author: Option<String>,
    pub body: Option<String>,
    pub moderation_hidden: bool,
    pub version_number: i32,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub has_replies: bool,
    pub like_count: i64,
    pub liked_by_viewer: bool,
}

#[server]
pub async fn thread(
    recipe: Uuid,
    parent: Option<i64>,
    after: i64,
) -> Result<Vec<Comment>, ServerFnError> {
    crate::security::enforce_request_budget(None, crate::security::RequestClass::PublicDiscovery)
        .map_err(|_| ServerFnError::new("Please try again later"))?;
    let viewer = crate::auth::session::extract_user_id_from_fullstack().await;
    read_thread(&crate::db::get_pool(), recipe, parent, after, viewer).await
}

#[cfg(feature = "server")]
pub(super) async fn read_thread(
    pool: &sqlx::PgPool,
    recipe: Uuid,
    parent: Option<i64>,
    after: i64,
    viewer: Option<Uuid>,
) -> Result<Vec<Comment>, ServerFnError> {
    sqlx::query_as::<_, Comment>("SELECT c.id, CASE WHEN NOT c.moderation_hidden THEN c.author_id END AS author_id,
        CASE WHEN NOT c.moderation_hidden THEN u.username END AS author,
        CASE WHEN NOT c.moderation_hidden THEN c.body END AS body,c.moderation_hidden,c.version_number,c.created_at,
        (SELECT count(*) FROM recipe_comment_likes l WHERE l.comment_id=c.id AND NOT c.moderation_hidden) AS like_count,
        EXISTS(SELECT 1 FROM recipe_comment_likes l WHERE l.comment_id=c.id AND NOT c.moderation_hidden AND l.user_id=$4) AS liked_by_viewer,
        EXISTS(SELECT 1 FROM recipe_comments child WHERE child.recipe_id=c.recipe_id AND child.parent_id=c.id) AS has_replies
        FROM recipe_comments c JOIN recipes r ON r.id=c.recipe_id LEFT JOIN users u ON u.id=c.author_id
        WHERE c.recipe_id=$1 AND r.visibility='public' AND c.parent_id IS NOT DISTINCT FROM $2 AND c.id>$3 ORDER BY c.id LIMIT 21")
        .bind(recipe).bind(parent).bind(after).bind(viewer).fetch_all(pool).await.map_err(|_| ServerFnError::new("Could not load comments"))
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
    SetLike {
        id: i64,
        liked: bool,
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
            sqlx::query("UPDATE recipe_comments SET body=$4 WHERE NOT moderation_hidden AND recipe_id=$1 AND id=$2 AND author_id=$3 RETURNING id")
                .bind(recipe).bind(id).bind(user).bind(validate(body)?).fetch_one(&mut *tx).await.map_err(failure)?;
        }
        Change::SetLike { id, liked } => {
            sqlx::query(
                "SELECT id FROM recipe_comments WHERE id=$1 AND recipe_id=$2 AND body IS NOT NULL AND NOT moderation_hidden",
            )
            .bind(id)
            .bind(recipe)
            .fetch_one(&mut *tx)
            .await
            .map_err(failure)?;
            let query = if liked {
                "INSERT INTO recipe_comment_likes (comment_id,user_id) VALUES ($1,$2) ON CONFLICT DO NOTHING"
            } else {
                "DELETE FROM recipe_comment_likes WHERE comment_id=$1 AND user_id=$2"
            };
            sqlx::query(query)
                .bind(id)
                .bind(user)
                .execute(&mut *tx)
                .await
                .map_err(failure)?;
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
            let (version, author_id): (i32, Option<Uuid>) = sqlx::query_as(
                "SELECT version_number, author_id FROM recipe_comments
                 WHERE recipe_id=$1 AND id=$2 AND body=$3 AND NOT moderation_hidden",
            )
            .bind(recipe)
            .bind(id)
            .bind(&observed_body)
            .fetch_optional(&mut *tx)
            .await
            .map_err(failure)?
            .ok_or_else(|| ServerFnError::new("Comment changed or unavailable"))?;
            let case_id: i64 = sqlx::query_scalar(
                "INSERT INTO comment_review_cases (recipe_id,comment_id,reported_parent_id)
                 SELECT recipe_id,id,parent_id FROM recipe_comments WHERE recipe_id=$1 AND id=$2
                 ON CONFLICT (comment_id) WHERE resolved_at IS NULL
                 DO UPDATE SET comment_id=EXCLUDED.comment_id RETURNING id",
            )
            .bind(recipe)
            .bind(id)
            .fetch_one(&mut *tx)
            .await
            .map_err(failure)?;
            let inserted = sqlx::query("INSERT INTO comment_reports (case_id,reporter_id,reason,reported_body,recipe_version,reported_author_id)
                         VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (case_id,reporter_id) DO NOTHING")
                .bind(case_id).bind(user).bind(reason).bind(observed_body).bind(version).bind(author_id)
                .execute(&mut *tx).await.map_err(failure)?;
            if inserted.rows_affected() > 0 {
                sqlx::query("UPDATE comment_review_cases SET revision=revision+1 WHERE id=$1")
                    .bind(case_id)
                    .execute(&mut *tx)
                    .await
                    .map_err(failure)?;
            }
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
            sqlx::query("DELETE FROM recipe_comment_likes WHERE comment_id=$1")
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(failure)?;
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
        let root = read_thread(&pool, recipe, None, 0, None)
            .await
            .unwrap()
            .remove(0);
        apply(&pool, users[1], recipe, publish(Some(root.id)))
            .await
            .unwrap();
        let reply = read_thread(&pool, recipe, Some(root.id), 0, None)
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
            read_thread(&pool, recipe, Some(root.id), 0, None)
                .await
                .unwrap()[0]
                .body,
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
        apply(&pool, users[1], recipe, Change::Delete { id: reply.id })
            .await
            .unwrap();
        assert!(report(users[1], "Deleted", "Original").await.is_err());
        let retained: i64 = sqlx::query_scalar("SELECT count(*) FROM comment_reports WHERE reported_body='Original' AND recipe_version=1 AND reported_author_id=$1")
            .bind(users[1]).fetch_one(&pool).await.unwrap();
        assert_eq!(retained, 4);
        sqlx::query("DELETE FROM users WHERE id=$1")
            .bind(users[1])
            .execute(&pool)
            .await
            .unwrap();
        let anonymized: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM comment_reports WHERE reported_author_id IS NULL",
        )
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(anonymized, 4);
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
        let root = read_thread(&pool, recipe, None, 0, None)
            .await
            .unwrap()
            .remove(0);
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
        let reply = read_thread(&pool, recipe, Some(root.id), 0, None)
            .await
            .unwrap()
            .remove(0);
        assert_eq!(reply.version_number, 3);
        let other: Uuid = sqlx::query_scalar("INSERT INTO users(username,display_name,email) VALUES ('reader','Reader','reader@example.test') RETURNING id").fetch_one(&pool).await.unwrap();
        let set_like =
            |actor, id, liked| apply(&pool, actor, recipe, Change::SetLike { id, liked });
        tokio::try_join!(
            set_like(user, reply.id, true),
            set_like(other, reply.id, true),
            set_like(user, reply.id, true)
        )
        .unwrap();
        for viewer in [None, Some(user), Some(other)] {
            let rows = read_thread(&pool, recipe, Some(root.id), 0, viewer)
                .await
                .unwrap();
            assert_eq!(rows[0].like_count, 2);
            assert_eq!(rows[0].liked_by_viewer, viewer.is_some());
        }
        set_like(user, reply.id, false).await.unwrap();
        set_like(user, reply.id, false).await.unwrap();
        let rows = read_thread(&pool, recipe, Some(root.id), 0, Some(user))
            .await
            .unwrap();
        assert_eq!(rows[0].like_count, 1);
        assert!(!rows[0].liked_by_viewer);
        set_like(other, root.id, true).await.unwrap();

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
            read_thread(&pool, recipe, None, 0, None).await.unwrap()[0].version_number,
            2
        );
        apply(&pool, user, recipe, Change::Delete { id: root.id })
            .await
            .unwrap();
        let deleted = read_thread(&pool, recipe, None, 0, None)
            .await
            .unwrap()
            .remove(0);
        assert_eq!(deleted.like_count, 0);
        assert!(set_like(other, root.id, true).await.is_err());
        assert!(deleted.body.is_none() && deleted.author_id.is_none() && deleted.has_replies);
        let leaf = read_thread(&pool, recipe, Some(reply.id), 0, None)
            .await
            .unwrap()
            .remove(0);
        apply(&pool, user, recipe, Change::Delete { id: leaf.id })
            .await
            .unwrap();
        assert!(read_thread(&pool, recipe, Some(reply.id), 0, None)
            .await
            .unwrap()
            .is_empty());
        sqlx::query("UPDATE recipes SET visibility='private' WHERE id=$1")
            .bind(recipe)
            .execute(&pool)
            .await
            .unwrap();
        assert!(read_thread(&pool, recipe, Some(root.id), 0, None)
            .await
            .unwrap()
            .is_empty());
        assert!(set_like(other, reply.id, false).await.is_err());
        assert!(apply(&pool, user, recipe, publish(None)).await.is_err());
    }
}
