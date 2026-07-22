use ame_api::auth::token::hash_secret;
use ame_api::http::db::convert_pool_to_ame_app;
use ame_api::http::{AppState, auth_extract_middleware, tasks};
use ame_api::progress::ProgressRepository;
use ame_api::progress_postgres::PgProgressRepository;
use ame_api::task_postgres::PgTaskSubmissionRepository;
use axum::{
    Router, middleware,
    routing::{patch, post},
};
use reqwest::StatusCode;
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx::migrate!("../db/migrations");

#[tokio::test]
async fn flink_task_review_creates_task_sourced_mastery_evidence() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping task evidence integration test; set AME_RUN_DB_TESTS=1");
        return;
    }
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&database_url)
        .await
        .expect("connect to clean PostgreSQL database");
    MIGRATOR.run(&pool).await.expect("apply migrations");

    let subject = Uuid::now_v7();
    let goal = Uuid::now_v7();
    let journey = Uuid::now_v7();
    let objective = Uuid::now_v7();
    let activity = Uuid::now_v7();
    let mut transaction = pool.begin().await.expect("begin fixture transaction");
    sqlx::query("SET CONSTRAINTS ALL DEFERRED")
        .execute(&mut *transaction)
        .await
        .expect("defer identity constraints");
    sqlx::query(
        "INSERT INTO tb_users (id, email, email_canonical, display_name, role)
         VALUES ($1, $2, $2, 'Flink Learner', 'learner')",
    )
    .bind(subject)
    .bind(format!("flink-task-{subject}@example.test"))
    .execute(&mut *transaction)
    .await
    .expect("insert learner");
    sqlx::query(
        "INSERT INTO tb_identities (id, identity_type, owner_user_id, label)
         VALUES ($1, 'human', $1, 'Flink Learner')",
    )
    .bind(subject)
    .execute(&mut *transaction)
    .await
    .expect("insert learner identity");
    sqlx::query(
        "INSERT INTO tb_learning_goals
             (id, subject_user_id, source_actor_id, raw_intent, normalized_statement, status)
         VALUES ($1, $2, $2, 'learn Flink', 'Learn Flink event-time processing', 'active')",
    )
    .bind(goal)
    .bind(subject)
    .execute(&mut *transaction)
    .await
    .expect("insert Flink goal");
    sqlx::query(
        "INSERT INTO tb_learning_journeys
             (id, goal_id, subject_user_id, source_actor_id, promise, status)
         VALUES ($1, $2, $3, $3, 'Build reliable Flink streaming intuition', 'active')",
    )
    .bind(journey)
    .bind(goal)
    .bind(subject)
    .execute(&mut *transaction)
    .await
    .expect("insert Flink journey");
    sqlx::query(
        "INSERT INTO tb_journey_objectives
             (id, journey_id, subject_user_id, verb, statement, success_criteria, order_index)
         VALUES ($1, $2, $3, 'diagnose', 'Explain why event time matters', 'Choose a sound watermark strategy', 0)",
    )
    .bind(objective)
    .bind(journey)
    .bind(subject)
    .execute(&mut *transaction)
    .await
    .expect("insert Flink objective");
    sqlx::query(
        "INSERT INTO tb_activities
             (id, journey_id, subject_user_id, source_actor_id, kind, title, order_index,
              payload_schema_version, payload, status, content_version, publication_status)
         VALUES ($1, $2, $3, $3, 'application', 'Choose a watermark strategy', 0, 1, $4, 'ready', 2, 'published')",
    )
    .bind(activity)
    .bind(journey)
    .bind(subject)
    .bind(serde_json::json!({
        "content": {
            "type": "scenario",
            "context": "Events arrive late from mobile devices.",
            "prompt": "What should the Flink job use?",
            "options": [{"id": "event-time", "label": "Event time with watermarks"}, {"id": "processing-time", "label": "Processing time only"}]
        }
    }))
    .execute(&mut *transaction)
    .await
    .expect("insert Flink task activity");
    sqlx::query("INSERT INTO tb_activity_objectives (activity_id, objective_id) VALUES ($1, $2)")
        .bind(activity)
        .bind(objective)
        .execute(&mut *transaction)
        .await
        .expect("link task objective");
    transaction.commit().await.expect("commit Flink fixture");

    let admin = Uuid::now_v7();
    let mut transaction = pool.begin().await.expect("begin admin fixture transaction");
    sqlx::query(
        "INSERT INTO tb_users (id, email, email_canonical, display_name, role)
         VALUES ($1, $2, $2, 'Flink Reviewer', 'admin')",
    )
    .bind(admin)
    .bind(format!("flink-reviewer-{admin}@example.test"))
    .execute(&mut *transaction)
    .await
    .expect("insert admin");
    sqlx::query(
        "INSERT INTO tb_identities (id, identity_type, owner_user_id, label)
         VALUES ($1, 'human', $1, 'Flink Reviewer')",
    )
    .bind(admin)
    .execute(&mut *transaction)
    .await
    .expect("insert admin identity");
    let learner_secret = "flink-learner-secret";
    let admin_secret = "flink-admin-secret";
    let learner_session_id = Uuid::now_v7();
    let admin_session_id = Uuid::now_v7();
    for (session_id, user_id, secret) in [
        (learner_session_id, subject, learner_secret),
        (admin_session_id, admin, admin_secret),
    ] {
        sqlx::query(
            "INSERT INTO tb_login_sessions (id, user_id, token_hash, expires_at)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(session_id)
        .bind(user_id)
        .bind(hash_secret(secret))
        .bind(OffsetDateTime::now_utc() + Duration::hours(1))
        .execute(&mut *transaction)
        .await
        .expect("insert test login session");
    }
    transaction.commit().await.expect("commit auth fixture");

    let pool_for_state = convert_pool_to_ame_app(&pool);
    let config = ame_api::config::Config::load().expect("load test config");
    let valkey = ame_api::config::create_valkey_pool(&config).expect("create test valkey pool");
    let state = AppState {
        pool: pool_for_state,
        config,
        limiter: std::sync::Arc::new(ame_api::ratelimit::RateLimiter::new(valkey.clone())),
        valkey,
    };
    let app = Router::new()
        .route(
            "/v1/tasks/{task_id}/submissions",
            post(tasks::start_submission),
        )
        .route(
            "/v1/task-submissions/{submission_id}/submit",
            post(tasks::submit_submission),
        )
        .route(
            "/v1/task-submissions/{submission_id}",
            axum::routing::get(tasks::get_submission),
        )
        .route(
            "/v1/task-submissions/{submission_id}/review",
            patch(tasks::review_submission),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_extract_middleware,
        ))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test server");
    let address = listener.local_addr().expect("read test server address");
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service())
            .await
            .expect("serve task test routes");
    });
    let client = reqwest::Client::new();
    let base_url = format!("http://{address}");
    let learner_token = format!("lgn_{learner_session_id}_{learner_secret}");
    let admin_token = format!("lgn_{admin_session_id}_{admin_secret}");

    let started = client
        .post(format!("{base_url}/v1/tasks/{activity}/submissions"))
        .bearer_auth(&learner_token)
        .json(&serde_json::json!({
            "contentVersion": 2,
            "response": {"optionId": "event-time"},
            "evaluationMethod": "agent"
        }))
        .send()
        .await
        .expect("start task over HTTP");
    assert_eq!(started.status(), StatusCode::CREATED);
    let started: serde_json::Value = started.json().await.expect("decode started task");
    let submission_id = started["id"].as_str().expect("submission id");
    let submitted = client
        .post(format!(
            "{base_url}/v1/task-submissions/{submission_id}/submit"
        ))
        .bearer_auth(&learner_token)
        .send()
        .await
        .expect("submit task over HTTP");
    assert_eq!(submitted.status(), StatusCode::OK);
    let reviewed = client
        .patch(format!(
            "{base_url}/v1/task-submissions/{submission_id}/review"
        ))
        .bearer_auth(&admin_token)
        .json(&serde_json::json!({
            "outcome": "reviewed",
            "score": 0.9,
            "feedback": {"note": "Correct event-time reasoning"}
        }))
        .send()
        .await
        .expect("review task over HTTP");
    assert_eq!(reviewed.status(), StatusCode::OK);
    let reviewed: serde_json::Value = reviewed.json().await.expect("decode reviewed task");
    assert_eq!(reviewed["reviewStatus"], "complete");
    assert_eq!(reviewed["score"], 0.9);
    let learner_read = client
        .get(format!("{base_url}/v1/task-submissions/{submission_id}"))
        .bearer_auth(&learner_token)
        .send()
        .await
        .expect("read reviewed task over HTTP");
    assert_eq!(learner_read.status(), StatusCode::OK);
    let learner_read: serde_json::Value =
        learner_read.json().await.expect("decode learner task read");
    assert_eq!(
        learner_read["feedback"]["note"],
        "Correct event-time reasoning"
    );

    let tasks = PgTaskSubmissionRepository::new(pool.clone());
    let reviewed_id = Uuid::parse_str(submission_id).expect("parse submission id");
    let context = tasks
        .evidence_context(reviewed_id)
        .await
        .expect("resolve evidence context");
    assert_eq!(context.score, 0.9);
    let progress = PgProgressRepository::new(pool);
    let snapshot = progress
        .snapshot(subject, journey, objective)
        .await
        .expect("calculate mastery snapshot");
    assert_eq!(snapshot.mastery, 0.9);
    assert_eq!(snapshot.evidence_count, 1);
}
