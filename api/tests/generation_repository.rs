use ame_api::domain::generation::{GenerationRepository, GenerationStatus, StartGenerationRun};
use ame_api::generation_postgres::PgGenerationRepository;
use serde_json::json;
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx::migrate!("../db/migrations");

#[tokio::test]
async fn postgres_generation_repository_preserves_retry_and_failure_semantics() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping PostgreSQL generation contract; set AME_RUN_DB_TESTS=1");
        return;
    }

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must point to a clean PostgreSQL database");
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("connect to PostgreSQL");
    MIGRATOR.run(&pool).await.expect("apply baseline");

    let subject = Uuid::now_v7();
    let other_subject = Uuid::now_v7();
    let mut transaction = pool.begin().await.expect("begin identity transaction");
    sqlx::query("SET CONSTRAINTS ALL DEFERRED")
        .execute(&mut *transaction)
        .await
        .expect("defer identity constraint");
    for user in [subject, other_subject] {
        sqlx::query(
            "INSERT INTO tb_users (id, email, email_canonical, display_name) VALUES ($1, $2, $2, $3)",
        )
        .bind(user)
        .bind(format!("generation-{user}@example.test"))
        .bind("Generation Learner")
        .execute(&mut *transaction)
        .await
        .expect("create learner");
        sqlx::query(
            "INSERT INTO tb_identities (id, identity_type, owner_user_id, label) VALUES ($1, 'human', $1, 'generation learner')",
        )
        .bind(user)
        .execute(&mut *transaction)
        .await
        .expect("create identity");
    }
    transaction.commit().await.expect("commit identities");

    let repository = PgGenerationRepository::new(pool);
    let input = StartGenerationRun {
        subject_user_id: subject,
        source_actor_id: subject,
        operation: "question.compose".into(),
        provider: Some("external-agent".into()),
        retry_key: Some(format!("retry-{subject}")),
        content_version: 1,
    };
    let first = repository.start(input.clone()).await.expect("start");
    let retry = repository.start(input).await.expect("retry");
    assert_eq!(first.id, retry.id);
    assert_eq!(first.status, GenerationStatus::Requested);

    repository
        .transition(subject, first.id, GenerationStatus::Running, None)
        .await
        .expect("running");
    let failed = repository
        .transition(
            subject,
            first.id,
            GenerationStatus::Failed,
            Some(json!({"code": "provider_unavailable"})),
        )
        .await
        .expect("failed");
    assert_eq!(failed.status, GenerationStatus::Failed);
    assert!(
        repository
            .transition(subject, first.id, GenerationStatus::Published, None)
            .await
            .is_err()
    );
    assert!(repository.get(other_subject, first.id).await.is_err());
}
