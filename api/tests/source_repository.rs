use ame_api::domain::source::{ImportSource, SourceError, SourceKind};
use ame_api::source::SourceRepository;
use ame_api::source_postgres::PgSourceRepository;
use sqlx::{migrate::Migrator, postgres::PgPoolOptions};
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx::migrate!("../db/migrations");

#[tokio::test]
async fn source_import_is_immutable_idempotent_and_owner_scoped() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping source integration test; set AME_RUN_DB_TESTS=1");
        return;
    }
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"))
        .await
        .expect("connect to PostgreSQL");
    MIGRATOR.run(&pool).await.expect("apply migrations");
    let subject = Uuid::now_v7();
    let other = Uuid::now_v7();
    for user in [subject, other] {
        let mut transaction = pool.begin().await.expect("begin identity");
        sqlx::query("SET CONSTRAINTS ALL DEFERRED")
            .execute(&mut *transaction)
            .await
            .expect("defer identity constraint");
        sqlx::query(
            "INSERT INTO tb_users (id, email, email_canonical, display_name)
             VALUES ($1, $2, $2, 'Source Learner')",
        )
        .bind(user)
        .bind(format!("source-{user}@example.test"))
        .execute(&mut *transaction)
        .await
        .expect("insert learner");
        sqlx::query(
            "INSERT INTO tb_identities (id, identity_type, owner_user_id, label)
             VALUES ($1, 'human', $1, 'Source Learner')",
        )
        .bind(user)
        .execute(&mut *transaction)
        .await
        .expect("insert identity");
        transaction.commit().await.expect("commit identity");
    }
    let repository = PgSourceRepository::new(pool);
    let input = ImportSource {
        subject_user_id: subject,
        source_kind: SourceKind::LocalFile,
        locator: "notes/flink.md".into(),
        media_type: "text/markdown".into(),
        content: b"# Event time\nWatermarks describe event-time progress.".to_vec(),
        retry_key: "flink-notes-v1".into(),
    };
    let (_, _, first) = repository.import(input.clone()).await.expect("import");
    let (_, _, retry) = repository.import(input.clone()).await.expect("retry");
    assert_eq!(first.id, retry.id);
    assert_eq!(first.content_sha256, retry.content_sha256);
    assert_eq!(
        repository.get_snapshot(subject, first.id).await.unwrap(),
        first
    );
    assert_eq!(
        repository.get_snapshot(other, first.id).await,
        Err(SourceError::NotFound)
    );
    assert_eq!(
        repository
            .import(ImportSource {
                content: b"different bytes".to_vec(),
                ..input
            })
            .await,
        Err(SourceError::RetryConflict)
    );

    let failed = repository
        .record_failure(
            subject,
            SourceKind::Url,
            "https://example.test/unavailable".into(),
            "unavailable-v1".into(),
            "source_fetch_failed".into(),
        )
        .await
        .expect("record failed import");
    let repeated = repository
        .record_failure(
            subject,
            SourceKind::Url,
            "https://example.test/unavailable".into(),
            "unavailable-v1".into(),
            "source_fetch_failed".into(),
        )
        .await
        .expect("repeat failed import");
    assert_eq!(failed, repeated);
    assert_eq!(failed.error_code.as_deref(), Some("source_fetch_failed"));
    assert_eq!(
        repository.list_imports(subject).await.unwrap().len(),
        2,
        "completed and failed imports remain inspectable"
    );
    assert!(repository.list_imports(other).await.unwrap().is_empty());
}
