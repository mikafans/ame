//! Planner integration tests.
//!
//! Gated behind `AME_RUN_DB_TESTS=1`; plain `make check` compiles this test
//! while keeping DB access opt-in.

use sqlx::PgPool;
use uuid::Uuid;

use ame_api::{
    bank::questions::{self as q_repo, QuestionInsert},
    domain::{
        question::{Judge, McPayload, Normalize, QuestionKind, ShortPayload},
        session::PlanItem,
    },
    engine::planner::{QuizPlanRequest, TagsMode, plan_quiz},
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
        .expect("run migrations for planner integration tests");

    pool
}

fn skip_if_no_db() -> bool {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!(
            "skipping planner DB integration test; set AME_RUN_DB_TESTS=1 and run make db-up"
        );
        return true;
    }
    false
}

async fn make_user(pool: &PgPool) -> Uuid {
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO tb_users (id, display_name, email, role) VALUES ($1, $2, $3, 'user')")
        .bind(id)
        .bind(format!("planner-test-{id}"))
        .bind(format!("planner-{id}@example.com"))
        .execute(pool)
        .await
        .unwrap();
    id
}

fn mc_insert(prompt: &str, tags: &[&str]) -> QuestionInsert {
    QuestionInsert {
        kind: QuestionKind::Mc,
        prompt: prompt.to_string(),
        payload: serde_json::to_value(McPayload {
            options: vec!["a".into(), "b".into(), "c".into()],
            correct_index: 1,
        })
        .unwrap(),
        explanation: None,
        points: Some(1),
        tags: tags.iter().map(|tag| tag.to_string()).collect(),
    }
}

fn short_insert(prompt: &str, tags: &[&str]) -> QuestionInsert {
    QuestionInsert {
        kind: QuestionKind::Short,
        prompt: prompt.to_string(),
        payload: serde_json::to_value(ShortPayload {
            accepted: vec!["answer".into()],
            normalize: Normalize::Exact,
            judge: Judge::Exact,
        })
        .unwrap(),
        explanation: None,
        points: Some(1),
        tags: tags.iter().map(|tag| tag.to_string()).collect(),
    }
}

#[tokio::test]
async fn quiz_planner_filters_live_questions_and_snapshots_mc_option_order() {
    if skip_if_no_db() {
        return;
    }

    let pool = setup_db().await;
    let user_id = make_user(&pool).await;
    let mut created = q_repo::create_questions(
        &pool,
        user_id,
        vec![
            mc_insert("planner live rust", &["rust", "async"]),
            short_insert("planner live rust text", &["rust"]),
            mc_insert("planner draft other", &["rust"]),
        ],
    )
    .await
    .unwrap();

    let draft = created.pop().unwrap();
    q_repo::promote_question(&pool, created[0].id)
        .await
        .unwrap();
    q_repo::promote_question(&pool, created[1].id)
        .await
        .unwrap();

    let plan = plan_quiz(
        &pool,
        user_id,
        &QuizPlanRequest {
            tags: vec!["Rust".to_string()],
            tags_mode: TagsMode::Any,
            difficulty_min: None,
            difficulty_max: None,
            count: 10,
            exclude_recent_hours: 0,
        },
    )
    .await
    .unwrap();

    let ids: Vec<Uuid> = plan
        .question_plan
        .items
        .iter()
        .map(|item| item.question_id)
        .collect();
    assert_eq!(ids.len(), 2);
    assert!(
        !ids.contains(&draft.id),
        "draft questions must not be planned"
    );
    assert!(plan.warning.is_some(), "short pool should be reported");
    assert!(plan.question_plan.items.iter().any(has_mc_option_order));
}

#[tokio::test]
async fn quiz_planner_excludes_recent_attempts_for_user() {
    if skip_if_no_db() {
        return;
    }

    let pool = setup_db().await;
    let user_id = make_user(&pool).await;
    let questions = q_repo::create_questions(
        &pool,
        user_id,
        vec![
            mc_insert("planner recent", &["rust"]),
            mc_insert("planner fresh", &["rust"]),
        ],
    )
    .await
    .unwrap();
    for question in &questions {
        q_repo::promote_question(&pool, question.id).await.unwrap();
    }

    sqlx::query(
        "INSERT INTO tb_attempts \
         (user_id, question_id, question_version, response, presentation, is_correct, score, \
          rating_before_user_avg, rating_before_question, user_tag_deltas, question_delta) \
         VALUES ($1, $2, $3, $4, $5, true, 1.0, 1200, 1400, '{}'::jsonb, 0)",
    )
    .bind(user_id)
    .bind(questions[0].id)
    .bind(questions[0].version)
    .bind(serde_json::json!({ "selected_position": 0 }))
    .bind(serde_json::json!({ "option_order": [0, 1, 2] }))
    .execute(&pool)
    .await
    .unwrap();

    let plan = plan_quiz(
        &pool,
        user_id,
        &QuizPlanRequest {
            tags: vec!["rust".to_string()],
            tags_mode: TagsMode::All,
            difficulty_min: None,
            difficulty_max: None,
            count: 10,
            exclude_recent_hours: 24,
        },
    )
    .await
    .unwrap();

    let ids: Vec<Uuid> = plan
        .question_plan
        .items
        .iter()
        .map(|item| item.question_id)
        .collect();
    assert!(!ids.contains(&questions[0].id));
    assert!(ids.contains(&questions[1].id));
}

fn has_mc_option_order(item: &PlanItem) -> bool {
    item.option_order
        .as_ref()
        .is_some_and(|order| order.len() == 3)
}
