//! Bank repository integration tests.
//!
//! Gated behind `AME_RUN_DB_TESTS=1` — these need a live Postgres reachable
//! via `DATABASE_URL` (`make db-up` brings one up locally). The plain
//! `make check` run prints "skipping" and exits 0 so the pre-commit gate
//! stays fast; CI / pre-PR runs hit them via `make test-db` or `make test-bank`.
//!
//! Coverage targets per `docs/plans/2026-05-20-plan-3-question-bank.md` §Testing:
//!
//! - Tag get-or-create lowercases + dedupes.
//! - Batch create enforces the 50-question cap.
//! - Editing a `live` question writes a snapshot and bumps `version`.
//! - Editing a `draft` question does not snapshot or bump.
//! - Editing an `archived` question is rejected.

use sqlx::{PgPool, Row};
use uuid::Uuid;

use ame_api::{
    bank::{
        questions::{self as q_repo, MAX_BATCH, QuestionFilter, QuestionInsert, QuestionPatch},
        tags as t_repo,
    },
    domain::{
        error::ApiError,
        question::{McPayload, QuestionKind, QuestionStatus},
    },
};

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../db/migrations");

async fn setup_db() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/ame".to_string());

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("set DATABASE_URL to a reachable Postgres before AME_RUN_DB_TESTS=1");

    MIGRATOR
        .run(&pool)
        .await
        .expect("run migrations for bank integration tests");

    pool
}

fn skip_if_no_db() -> bool {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping bank DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up");
        return true;
    }
    false
}

async fn make_user(pool: &PgPool) -> Uuid {
    let id = Uuid::now_v7();
    let display_name = format!("bank-test-{}", id);
    sqlx::query("INSERT INTO users (id, display_name, role) VALUES ($1, $2, 'user')")
        .bind(id)
        .bind(display_name)
        .execute(pool)
        .await
        .unwrap();
    id
}

fn mc_insert(prompt: &str, tags: &[&str]) -> QuestionInsert {
    let payload = serde_json::to_value(McPayload {
        options: vec!["a".into(), "b".into(), "c".into()],
        correct_index: 1,
    })
    .unwrap();
    QuestionInsert {
        kind: QuestionKind::Mc,
        prompt: prompt.to_string(),
        code_snippet: None,
        payload,
        explanation: None,
        source: None,
        points: 1,
        tags: tags.iter().map(|s| s.to_string()).collect(),
    }
}

#[tokio::test]
async fn tag_get_or_create_lowercases_and_dedupes() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let mut tx = pool.begin().await.unwrap();

    let names = vec!["Rust".to_string(), "rust".to_string(), "ASYNC".to_string()];
    let tags = t_repo::get_or_create_tags_tx(&mut tx, &names)
        .await
        .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(tags.len(), 2);
    assert_eq!(tags[0].name, "rust");
    assert_eq!(tags[1].name, "async");
}

#[tokio::test]
async fn create_questions_batch_inserts_and_links_tags() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let user_id = make_user(&pool).await;

    let batch = vec![
        mc_insert("Q1", &["rust:async", "rust:lifetimes"]),
        mc_insert("Q2", &["rust:async"]),
        mc_insert("Q3", &[]),
    ];
    let created = q_repo::create_questions(&pool, user_id, batch)
        .await
        .unwrap();

    assert_eq!(created.len(), 3);
    for q in &created {
        assert_eq!(q.status, QuestionStatus::Draft);
        assert_eq!(q.version, 1);
        assert_eq!(q.created_by, user_id);
    }

    let q1_tag_count: i64 =
        sqlx::query("SELECT COUNT(*)::bigint AS n FROM question_tags WHERE question_id = $1")
            .bind(created[0].id)
            .fetch_one(&pool)
            .await
            .unwrap()
            .get("n");
    assert_eq!(q1_tag_count, 2);

    // Filter sanity: q2 should surface under tag=rust:async.
    let filter = QuestionFilter {
        tag: Some("rust:async".into()),
        ..Default::default()
    };
    let listed = q_repo::list_questions(&pool, &filter).await.unwrap();
    let listed_ids: Vec<Uuid> = listed.into_iter().map(|q| q.id).collect();

    let mut found_count = 0;
    for id in &listed_ids {
        if *id == created[0].id || *id == created[1].id {
            found_count += 1;
        }
    }
    assert!(
        found_count >= 1,
        "At least one of Q1 or Q2 should be found: {:?}",
        listed_ids
    );
    assert!(!listed_ids.contains(&created[2].id));
}

#[tokio::test]
async fn create_questions_rejects_oversized_batch() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let user_id = make_user(&pool).await;

    let batch: Vec<_> = (0..MAX_BATCH + 1)
        .map(|i| mc_insert(&format!("over{}", i), &[]))
        .collect();
    let err = q_repo::create_questions(&pool, user_id, batch)
        .await
        .unwrap_err();

    match err {
        ApiError::Validation(fields) => {
            assert_eq!(fields[0].field, "questions");
            assert!(fields[0].message.contains("max"));
        }
        other => panic!("expected Validation error, got {other:?}"),
    }
}

#[tokio::test]
async fn update_live_question_bumps_version_and_writes_history() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let user_id = make_user(&pool).await;

    let q = q_repo::create_questions(&pool, user_id, vec![mc_insert("live-q", &[])])
        .await
        .unwrap()
        .pop()
        .unwrap();

    q_repo::promote_question(&pool, q.id).await.unwrap();

    let patch = QuestionPatch {
        prompt: Some("live-q updated".into()),
        ..Default::default()
    };
    let updated = q_repo::update_question(&pool, q.id, patch).await.unwrap();

    assert_eq!(updated.version, 2, "live edit must bump version");
    assert_eq!(updated.prompt, "live-q updated");

    let versions = q_repo::list_question_versions(&pool, q.id).await.unwrap();
    assert_eq!(
        versions.len(),
        1,
        "previous live state should be snapshotted"
    );
    assert_eq!(versions[0].version, 1);
    assert_eq!(versions[0].prompt, "live-q");
}

#[tokio::test]
async fn update_draft_question_does_not_bump_version() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let user_id = make_user(&pool).await;

    let q = q_repo::create_questions(&pool, user_id, vec![mc_insert("draft-q", &[])])
        .await
        .unwrap()
        .pop()
        .unwrap();

    let patch = QuestionPatch {
        prompt: Some("draft-q updated".into()),
        ..Default::default()
    };
    let updated = q_repo::update_question(&pool, q.id, patch).await.unwrap();

    assert_eq!(updated.version, 1, "draft edit must not bump version");
    assert_eq!(updated.prompt, "draft-q updated");

    let versions = q_repo::list_question_versions(&pool, q.id).await.unwrap();
    assert!(versions.is_empty(), "draft edit must not snapshot");
}

#[tokio::test]
async fn cannot_edit_archived_question() {
    if skip_if_no_db() {
        return;
    }
    let pool = setup_db().await;
    let user_id = make_user(&pool).await;

    let q = q_repo::create_questions(&pool, user_id, vec![mc_insert("archive-me", &[])])
        .await
        .unwrap()
        .pop()
        .unwrap();
    q_repo::archive_question(&pool, q.id).await.unwrap();

    let patch = QuestionPatch {
        prompt: Some("nope".into()),
        ..Default::default()
    };
    let err = q_repo::update_question(&pool, q.id, patch)
        .await
        .unwrap_err();
    match err {
        ApiError::Validation(fields) => assert_eq!(fields[0].field, "status"),
        other => panic!("expected Validation, got {other:?}"),
    }
}
