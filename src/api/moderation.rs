use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Queue {
    Waiting,
    Mine,
    Reviewing,
    Resolved,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Case {
    pub id: i64,
    pub title: String,
    pub excerpt: String,
    pub status: String,
    pub queued_at: chrono::DateTime<chrono::Utc>,
    pub reports: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Evidence {
    pub author: Option<String>,
    pub reported_body: String,
    pub recipe_version: i32,
    pub reason: String,
    pub reported_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Entry {
    pub actor: String,
    pub action: String,
    pub rationale: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct CaseInfo {
    pub title: String,
    pub status: String,
    pub revision: i64,
    pub claimant: Option<Uuid>,
    pub reviewer: Option<String>,
    pub body: Option<String>,
    pub parent: Option<String>,
    pub parent_id: Option<i64>,
    pub hidden: bool,
}

#[derive(Clone, Serialize, Deserialize, PartialEq)]
pub struct Detail {
    pub info: CaseInfo,
    pub evidence: Vec<Evidence>,
    pub history: Vec<Entry>,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum Action {
    Claim,
    Release,
    Dismiss,
    Uphold,
    UpholdAndHide,
}

#[cfg(feature = "server")]
fn permitted(user: Uuid, configured: &str) -> bool {
    configured
        .split(',')
        .any(|entry| entry.trim().parse::<Uuid>() == Ok(user))
}

#[cfg(feature = "server")]
async fn moderator(expected: Option<Uuid>) -> Result<Uuid, ServerFnError> {
    let user = crate::auth::account_binding::require_form_account(expected).await?;
    if !permitted(
        user,
        &std::env::var("NOMS_COMMENT_MODERATOR_IDS").unwrap_or_default(),
    ) {
        return Err(ServerFnError::new("Moderator access required"));
    }
    Ok(user)
}

#[server]
pub async fn can_review(expected: Option<Uuid>) -> Result<bool, ServerFnError> {
    Ok(moderator(expected).await.is_ok())
}

#[cfg(feature = "server")]
fn db_error(_: sqlx::Error) -> ServerFnError {
    ServerFnError::new("Moderation service unavailable")
}

pub const PAGE_SIZE: usize = 20;

#[server]
pub async fn list_cases(
    expected: Option<Uuid>,
    queue: Queue,
    page: u32,
) -> Result<Vec<Case>, ServerFnError> {
    let actor = moderator(expected).await?;
    let filter = match queue {
        Queue::Waiting => "c.status IN ('queued','flagged','unflagged','uncertain')",
        Queue::Reviewing => "c.status='reviewing'",
        Queue::Mine => "c.status='reviewing' AND c.claimant=$1",
        Queue::Resolved => "c.resolved_at IS NOT NULL",
    };
    let order = if queue == Queue::Resolved {
        "c.resolved_at DESC, c.id DESC"
    } else {
        "c.queued_at, c.id"
    };
    // Only fixed SQL fragments are selected above. The actor and pagination offset are bound.
    let sql = format!("SELECT c.id, r.title, c.status::text AS status, c.queued_at,
        (SELECT count(*) FROM comment_reports WHERE case_id=c.id) AS reports,
        COALESCE((SELECT left(reported_body,160) FROM comment_reports WHERE case_id=c.id ORDER BY id LIMIT 1),'') AS excerpt
        FROM comment_review_cases c JOIN recipes r ON r.id=c.recipe_id
        WHERE $1::uuid IS NOT NULL AND c.queued_at IS NOT NULL AND {filter}
        ORDER BY {order} LIMIT 21 OFFSET $2");
    sqlx::query_as(&sql)
        .bind(actor)
        .bind(i64::from(page) * 20)
        .fetch_all(&crate::db::get_pool())
        .await
        .map_err(db_error)
}

#[server]
pub async fn case_detail(
    expected: Option<Uuid>,
    id: i64,
    evidence_page: u32,
) -> Result<Detail, ServerFnError> {
    moderator(expected).await?;
    let mut tx = crate::db::get_pool().begin().await.map_err(db_error)?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
    let info = sqlx::query_as("SELECT r.title,c.status::text AS status,c.revision,c.claimant,u.username AS reviewer,
        m.body,COALESCE(m.parent_id,c.reported_parent_id) AS parent_id,p.body AS parent,COALESCE(m.moderation_hidden,false) AS hidden
        FROM comment_review_cases c JOIN recipes r ON r.id=c.recipe_id LEFT JOIN users u ON u.id=c.claimant
        LEFT JOIN recipe_comments m ON m.id=c.comment_id LEFT JOIN recipe_comments p ON p.id=COALESCE(m.parent_id,c.reported_parent_id)
        WHERE c.id=$1 AND c.queued_at IS NOT NULL").bind(id).fetch_optional(&mut *tx).await.map_err(db_error)?
        .ok_or_else(||ServerFnError::new("Case unavailable"))?;
    let evidence = sqlx::query_as(
        "SELECT u.username AS author,r.reported_body,r.recipe_version,r.reason,r.reported_at FROM comment_reports r LEFT JOIN users u ON u.id=r.reported_author_id
        WHERE case_id=$1 ORDER BY reported_body,recipe_version,r.id LIMIT 21 OFFSET $2",
    )
    .bind(id)
    .bind(i64::from(evidence_page) * 20)
    .fetch_all(&mut *tx)
    .await
    .map_err(db_error)?;
    let history = sqlx::query_as("SELECT COALESCE(u.username,'Former moderator') AS actor,e.action,e.rationale,e.created_at
        FROM comment_review_events e LEFT JOIN users u ON u.id=e.actor_id WHERE e.case_id=$1 ORDER BY e.id DESC")
        .bind(id).fetch_all(&mut *tx).await.map_err(db_error)?;
    tx.commit().await.map_err(db_error)?;
    Ok(Detail {
        info,
        evidence,
        history,
    })
}

#[server]
pub async fn review(
    expected: Option<Uuid>,
    id: i64,
    revision: i64,
    action: Action,
    rationale: String,
    observed: Option<String>,
) -> Result<(), ServerFnError> {
    let actor = moderator(expected).await?;
    crate::security::enforce_user_request_budget(actor, crate::security::RequestClass::Mutation)
        .map_err(|_| ServerFnError::new("Please try again later"))?;
    transition(
        &crate::db::get_pool(),
        actor,
        id,
        revision,
        action,
        &rationale,
        observed.as_deref(),
    )
    .await
}

#[cfg(feature = "server")]
async fn transition(
    pool: &sqlx::PgPool,
    actor: Uuid,
    id: i64,
    revision: i64,
    action: Action,
    rationale: &str,
    observed: Option<&str>,
) -> Result<(), ServerFnError> {
    let rationale = rationale.trim();
    if (action != Action::Claim && rationale.is_empty()) || rationale.chars().count() > 2000 {
        return Err(ServerFnError::new(
            "Enter a reason of up to 2000 characters",
        ));
    }
    let mut tx = pool.begin().await.map_err(db_error)?;
    // Take the same recipe lock as public comment mutations before locking the case.
    sqlx::query(
        "SELECT r.id FROM recipes r JOIN comment_review_cases c ON c.recipe_id=r.id
        WHERE c.id=$1 AND c.queued_at IS NOT NULL FOR UPDATE OF r",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await
    .map_err(db_error)?;
    let (status,claimant,current,comment,body): (String,Option<Uuid>,i64,i64,Option<String>) =
        sqlx::query_as("SELECT c.status::text,c.claimant,c.revision,c.comment_id,m.body FROM comment_review_cases c
        LEFT JOIN recipe_comments m ON m.id=c.comment_id WHERE c.id=$1 FOR UPDATE OF c")
        .bind(id).fetch_one(&mut *tx).await.map_err(db_error)?;
    if revision != current {
        return Err(ServerFnError::new(
            "Case changed. Review the latest details.",
        ));
    }
    let mine = status == "reviewing" && claimant == Some(actor);
    let (next, event) = match action {
        Action::Claim
            if matches!(
                status.as_str(),
                "queued" | "flagged" | "unflagged" | "uncertain"
            ) =>
        {
            ("reviewing", "claimed")
        }
        Action::Release if status == "reviewing" => ("queued", "released"),
        Action::Dismiss if mine => ("dismissed", "dismissed"),
        Action::Uphold if mine => ("upheld", "upheld"),
        Action::UpholdAndHide if mine && body.is_some() => ("upheld", "upheld and hid comment"),
        _ => return Err(ServerFnError::new("Action unavailable for this case")),
    };
    let resolved = matches!(
        action,
        Action::Uphold | Action::UpholdAndHide | Action::Dismiss
    );
    if resolved && body.as_deref() != observed {
        return Err(ServerFnError::new(
            "Comment changed. Review the latest text.",
        ));
    }
    if action == Action::UpholdAndHide {
        sqlx::query("UPDATE recipe_comments SET moderation_hidden=true WHERE id=$1")
            .bind(comment)
            .execute(&mut *tx)
            .await
            .map_err(db_error)?;
    }
    sqlx::query("UPDATE comment_review_cases SET status=$2::text::comment_review_status,claimant=$3,revision=revision+1,
        resolved_at=CASE WHEN $4 THEN now() ELSE NULL END WHERE id=$1")
        .bind(id).bind(next).bind((action==Action::Claim).then_some(actor)).bind(resolved).execute(&mut *tx).await.map_err(db_error)?;
    sqlx::query(
        "INSERT INTO comment_review_events(case_id,actor_id,action,rationale) VALUES ($1,$2,$3,$4)",
    )
    .bind(id)
    .bind(actor)
    .bind(event)
    .bind(rationale)
    .execute(&mut *tx)
    .await
    .map_err(db_error)?;
    tx.commit().await.map_err(db_error)
}

#[cfg(all(test, feature = "server"))]
mod tests {
    use super::*;

    #[test]
    fn permission_requires_an_explicit_account_uuid() {
        let moderator = Uuid::new_v4();
        let member = Uuid::new_v4();
        assert!(permitted(moderator, &format!("unknown, {moderator}")));
        assert!(!permitted(member, &moderator.to_string()));
        assert!(!permitted(member, ""));
    }

    #[tokio::test]
    async fn moderation_claim_decision_and_public_visibility() {
        let (_database, pool) = crate::test_utils::setup_test_db().await;
        let users: Vec<Uuid> = sqlx::query_scalar("INSERT INTO users(username,display_name,email)
            SELECT 'mod'||n,'Moderator '||n,n||'@example.test' FROM generate_series(1,2) n RETURNING id")
            .fetch_all(&pool).await.unwrap();
        let recipe: Uuid=sqlx::query_scalar("INSERT INTO recipes(user_id,title,visibility) VALUES($1,'Baking','public') RETURNING id")
            .bind(users[0]).fetch_one(&pool).await.unwrap();
        sqlx::query(
            "INSERT INTO recipe_versions(recipe_id,version_number,snapshot) VALUES($1,1,'{}')",
        )
        .bind(recipe)
        .execute(&pool)
        .await
        .unwrap();
        let comment = |parent: Option<i64>| {
            sqlx::query_scalar::<_,i64>("INSERT INTO recipe_comments(recipe_id,version_number,request_id,body,parent_id) VALUES($1,1,gen_random_uuid(),'Example',$2) RETURNING id").bind(recipe).bind(parent).fetch_one(&pool)
        };
        let root = comment(None).await.unwrap();
        let child = comment(Some(root)).await.unwrap();
        let case:i64=sqlx::query_scalar("INSERT INTO comment_review_cases(recipe_id,comment_id,status,queued_at) VALUES($1,$2,'queued',now()) RETURNING id").bind(recipe).bind(root).fetch_one(&pool).await.unwrap();
        let (a, b) = tokio::join!(
            transition(&pool, users[0], case, 0, Action::Claim, "", None),
            transition(&pool, users[1], case, 0, Action::Claim, "", None)
        );
        assert_ne!(a.is_ok(), b.is_ok());
        let (owner, other) = if a.is_ok() {
            (users[0], users[1])
        } else {
            (users[1], users[0])
        };
        assert!(transition(
            &pool,
            other,
            case,
            1,
            Action::Dismiss,
            "Reviewed",
            Some("Example")
        )
        .await
        .is_err());
        assert!(transition(
            &pool,
            owner,
            case,
            1,
            Action::UpholdAndHide,
            "Advertising",
            Some("Older text")
        )
        .await
        .is_err());
        transition(
            &pool,
            other,
            case,
            1,
            Action::Release,
            "Reviewer unavailable",
            None,
        )
        .await
        .unwrap();
        transition(&pool, other, case, 2, Action::Claim, "", None)
            .await
            .unwrap();
        transition(
            &pool,
            other,
            case,
            3,
            Action::UpholdAndHide,
            "Advertising",
            Some("Example"),
        )
        .await
        .unwrap();
        assert!(transition(
            &pool,
            other,
            case,
            3,
            Action::Dismiss,
            "Retry",
            Some("Example")
        )
        .await
        .is_err());
        let result:(String,bool,i64)=sqlx::query_as("SELECT status::text,resolved_at IS NOT NULL,(SELECT count(*) FROM comment_review_events WHERE case_id=$1) FROM comment_review_cases WHERE id=$1")
            .bind(case).fetch_one(&pool).await.unwrap();
        assert_eq!(result, ("upheld".into(), true, 4));
        let rows = crate::api::comments::read_thread(&pool, recipe, None, 0, None)
            .await
            .unwrap();
        assert!(rows[0].moderation_hidden && rows[0].body.is_none() && rows[0].has_replies);
        assert_eq!(
            crate::api::comments::read_thread(&pool, recipe, Some(root), 0, None)
                .await
                .unwrap()[0]
                .id,
            child
        );
    }
}
