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
