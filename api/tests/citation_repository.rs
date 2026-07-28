use ame_api::{
    citation::CitationRepository,
    citation_postgres::PgCitationRepository,
    domain::{
        citation::{
            CitationError, CreateCitation, ExtractionMethod, GroundingStatus, LicenseStatus,
        },
        source::{ImportSource, SourceKind},
    },
    source::SourceRepository,
    source_postgres::PgSourceRepository,
};
use sqlx::{migrate::Migrator, postgres::PgPoolOptions};
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx::migrate!("../db/migrations");

#[tokio::test]
async fn citation_certifies_exact_snapshot_range_before_publication() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping citation integration test; set AME_RUN_DB_TESTS=1");
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
             VALUES ($1, $2, $2, 'Citation Learner')",
        )
        .bind(user)
        .bind(format!("citation-{user}@example.test"))
        .execute(&mut *transaction)
        .await
        .expect("insert learner");
        sqlx::query(
            "INSERT INTO tb_identities (id, identity_type, owner_user_id, label)
             VALUES ($1, 'human', $1, 'Citation Learner')",
        )
        .bind(user)
        .execute(&mut *transaction)
        .await
        .expect("insert identity");
        transaction.commit().await.expect("commit identity");
    }
    let (_, _, snapshot) = PgSourceRepository::new(pool.clone())
        .import(ImportSource {
            subject_user_id: subject,
            source_kind: SourceKind::Document,
            locator: "event-time-guide".into(),
            media_type: "text/plain".into(),
            content: b"Event time uses timestamps embedded in each event.".to_vec(),
            retry_key: "event-time-guide-v1".into(),
        })
        .await
        .expect("import source");
    let repository = PgCitationRepository::new(pool);
    let input = CreateCitation {
        subject_user_id: subject,
        snapshot_id: snapshot.id,
        start_byte: 0,
        end_byte: 10,
        quote: "Event time".into(),
        extraction_method: ExtractionMethod::ExactQuote,
        grounding_status: GroundingStatus::Supported,
        grounding_note: "Exact definition supports the activity claim.".into(),
        license_status: LicenseStatus::Allowed,
        license_name: Some("CC BY 4.0".into()),
        license_url: None,
    };
    let citation = repository.create(input.clone()).await.expect("citation");
    repository
        .certify_publication(subject, &[citation.id])
        .await
        .expect("certify publication");
    assert_eq!(
        repository.get(other, citation.id).await,
        Err(CitationError::NotFound)
    );

    let mut replaced = input.clone();
    replaced.start_byte = 11;
    replaced.end_byte = 15;
    replaced.quote = "clock".into();
    assert_eq!(
        repository.create(replaced).await,
        Err(CitationError::QuoteMismatch)
    );

    let mut contradicted = input;
    contradicted.start_byte = 11;
    contradicted.end_byte = 15;
    contradicted.quote = "uses".into();
    contradicted.grounding_status = GroundingStatus::Contradicted;
    let contradicted = repository
        .create(contradicted)
        .await
        .expect("contradicted certificate");
    assert_eq!(
        repository
            .certify_publication(subject, &[contradicted.id])
            .await,
        Err(CitationError::NotGrounded)
    );
    assert_eq!(
        repository
            .certify_publication(subject, &[Uuid::now_v7()])
            .await,
        Err(CitationError::NotFound)
    );
}
