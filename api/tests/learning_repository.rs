use ame_api::learning::{LearningContractFixtures, exercise_goal_and_journey_contract};
use ame_api::learning_postgres::PgLearningRepository;
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx::migrate!("../db/migrations");

#[tokio::test]
async fn postgres_repository_satisfies_goal_and_journey_contract() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping PostgreSQL contract test; set AME_RUN_DB_TESTS=1");
        return;
    }

    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must point to a clean PostgreSQL database");
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await
        .expect("connect to clean PostgreSQL database");
    MIGRATOR
        .run(&pool)
        .await
        .expect("apply clean learning baseline");

    let subject = Uuid::now_v7();
    let mut transaction = pool.begin().await.expect("begin identity transaction");
    sqlx::query("SET CONSTRAINTS ALL DEFERRED")
        .execute(&mut *transaction)
        .await
        .expect("defer circular identity constraint");
    sqlx::query(
        "INSERT INTO tb_users (id, email, email_canonical, display_name, role)
         VALUES ($1, $2, $2, $3, 'learner')",
    )
    .bind(subject)
    .bind(format!("contract-{subject}@example.test"))
    .bind("Contract Learner")
    .execute(&mut *transaction)
    .await
    .expect("create contract learner");
    sqlx::query(
        "INSERT INTO tb_identities (id, identity_type, owner_user_id, label)
         VALUES ($1, 'human', $1, 'contract learner')",
    )
    .bind(subject)
    .execute(&mut *transaction)
    .await
    .expect("create human actor identity");
    transaction
        .commit()
        .await
        .expect("commit contract identity");

    let repository = PgLearningRepository::new(pool);
    exercise_goal_and_journey_contract(
        &repository,
        LearningContractFixtures {
            subject_user_id: subject,
            other_subject_user_id: Uuid::now_v7(),
            source_actor_id: subject,
        },
    )
    .await;
}
