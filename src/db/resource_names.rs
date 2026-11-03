//! Identify only the unique index that owns the named-resource invariant.
pub(super) fn is_name_conflict(error: &sqlx::Error, index: &str) -> bool {
    error.as_database_error().is_some_and(|database| {
        database.is_unique_violation() && database.constraint() == Some(index)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils;
    use sqlx::PgPool;
    use uuid::Uuid;

    async fn insert(pool: &PgPool, table: &str, owner: Uuid, name: &str) -> Result<Uuid, sqlx::Error> {
        // Table names are fixed test constants, never user input.
        let query = if table == "collections" {
            "INSERT INTO collections(owner_id,name) VALUES($1,$2) RETURNING id".to_string()
        } else {
            format!(
                "INSERT INTO {table}(owner_id,name,request_id,request_hash)
                 VALUES($1,$2,gen_random_uuid(),repeat('0',64)) RETURNING id"
            )
        };
        sqlx::query_scalar(&query)
            .bind(owner)
            .bind(name)
            .fetch_one(pool)
            .await
    }

    #[tokio::test]
    async fn direct_resource_writes_enforce_owner_names_and_allow_other_owners() {
        let (_database, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        let other = test_utils::insert_test_user(&pool).await;
        for (table, index) in [
            ("collections", "idx_collections_owner_name_unique"),
            ("grocery_lists", "idx_grocery_lists_owner_name_unique"),
            ("kitchen_profiles", "idx_kitchen_profiles_owner_name_unique"),
        ] {
            let id = insert(&pool, table, owner.id, "Weekly essentials").await.unwrap();
            let error = insert(&pool, table, owner.id, "  WEEKLY\t essentials  ").await.unwrap_err();
            assert!(is_name_conflict(&error, index));
            assert!(!is_name_conflict(&error, "unrelated_index"));
            insert(&pool, table, other.id, "Weekly essentials").await.unwrap();
            let second = insert(&pool, table, owner.id, "Other").await.unwrap();
            let query = format!("UPDATE {table} SET name=$1 WHERE id=$2");
            let error = sqlx::query(&query)
                .bind("weekly  essentials").bind(second).execute(&pool).await.unwrap_err();
            assert!(is_name_conflict(&error, index));
            sqlx::query(&query).bind(" WEEKLY essentials ").bind(id).execute(&pool).await.unwrap();
            sqlx::query(&format!("DELETE FROM {table} WHERE id=$1"))
                .bind(id).execute(&pool).await.unwrap();
            insert(&pool, table, owner.id, "Weekly essentials").await.unwrap();
        }
        assert!(!is_name_conflict(&sqlx::Error::RowNotFound, "anything"));
    }

    #[tokio::test]
    async fn concurrent_direct_resource_creates_have_one_winner() {
        let (_database, pool) = test_utils::setup_test_db().await;
        let owner = test_utils::insert_test_user(&pool).await;
        for (table, index) in [
            ("collections", "idx_collections_owner_name_unique"),
            ("grocery_lists", "idx_grocery_lists_owner_name_unique"),
            ("kitchen_profiles", "idx_kitchen_profiles_owner_name_unique"),
        ] {
            let (a, b) = tokio::join!(
                insert(&pool, table, owner.id, "Weekend supplies"),
                insert(&pool, table, owner.id, " WEEKEND  supplies ")
            );
            assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
            let error = a.err().or_else(|| b.err()).unwrap();
            assert!(is_name_conflict(&error, index));
        }
    }
}
