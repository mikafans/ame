use ame_api::domain::review::ReviewRating;
use ame_api::review::{RateReview, ReviewRepository};
use ame_api::review_postgres::PgReviewRepository;
use sqlx::{PgPool, migrate::Migrator, postgres::PgPoolOptions};
use time::OffsetDateTime;
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx::migrate!("../db/migrations");

#[tokio::test]
async fn evidence_seeds_an_owner_scoped_due_review_and_rating_reschedules_it() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping review integration test; set AME_RUN_DB_TESTS=1");
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
    let (journey, objective, activity, evidence) = seed_fixture(&pool, subject).await;
    let repository = PgReviewRepository::new(pool);

    let due = repository
        .list_due(subject, None)
        .await
        .expect("list due review");
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].journey_id, journey);
    assert_eq!(due[0].objective_id, objective);
    assert_eq!(due[0].activity_id, activity);
    assert!(
        repository
            .list_due(other, None)
            .await
            .expect("other learner due queue")
            .is_empty()
    );
    assert_eq!(
        repository
            .seed_from_evidence(subject, evidence)
            .await
            .expect("idempotent seed")
            .id,
        due[0].id
    );

    let rated = repository
        .rate(RateReview {
            subject_user_id: subject,
            review_item_id: due[0].id,
            rating: ReviewRating::Good,
            reviewed_at: OffsetDateTime::now_utc(),
        })
        .await
        .expect("rate review");
    assert_eq!(rated.review_count, 1);
    assert!(rated.interval_days >= 1);
    assert!(rated.due_at > rated.last_reviewed_at.expect("reviewed timestamp"));
}

async fn seed_fixture(pool: &PgPool, subject: Uuid) -> (Uuid, Uuid, Uuid, Uuid) {
    let goal = Uuid::now_v7();
    let journey = Uuid::now_v7();
    let objective = Uuid::now_v7();
    let activity = Uuid::now_v7();
    let session = Uuid::now_v7();
    let attempt = Uuid::now_v7();
    let evidence = Uuid::now_v7();
    let mut transaction = pool.begin().await.expect("begin fixture");
    sqlx::query("SET CONSTRAINTS ALL DEFERRED")
        .execute(&mut *transaction)
        .await
        .expect("defer identity constraint");
    sqlx::query(
        "INSERT INTO tb_users (id, email, email_canonical, display_name)
         VALUES ($1, $2, $2, 'Review Learner')",
    )
    .bind(subject)
    .bind(format!("review-{subject}@example.test"))
    .execute(&mut *transaction)
    .await
    .expect("insert learner");
    sqlx::query(
        "INSERT INTO tb_identities (id, identity_type, owner_user_id, label)
         VALUES ($1, 'human', $1, 'Review Learner')",
    )
    .bind(subject)
    .execute(&mut *transaction)
    .await
    .expect("insert identity");
    sqlx::query(
        "INSERT INTO tb_learning_goals
             (id, subject_user_id, source_actor_id, raw_intent, normalized_statement, status)
         VALUES ($1, $2, $2, 'retain Flink', 'Retain Flink concepts', 'active')",
    )
    .bind(goal)
    .bind(subject)
    .execute(&mut *transaction)
    .await
    .expect("insert goal");
    sqlx::query(
        "INSERT INTO tb_learning_journeys
             (id, goal_id, subject_user_id, source_actor_id, promise, status)
         VALUES ($1, $2, $3, $3, 'Retain event-time reasoning', 'active')",
    )
    .bind(journey)
    .bind(goal)
    .bind(subject)
    .execute(&mut *transaction)
    .await
    .expect("insert journey");
    sqlx::query(
        "INSERT INTO tb_journey_objectives
             (id, journey_id, subject_user_id, verb, statement, success_criteria, order_index)
         VALUES ($1, $2, $3, 'recall', 'Explain watermarks', 'Recall without hints', 0)",
    )
    .bind(objective)
    .bind(journey)
    .bind(subject)
    .execute(&mut *transaction)
    .await
    .expect("insert objective");
    sqlx::query(
        "INSERT INTO tb_activities
             (id, journey_id, subject_user_id, source_actor_id, kind, title, order_index,
              payload_schema_version, content_version, publication_status, payload, status)
         VALUES ($1, $2, $3, $3, 'practice', 'Watermark recall', 0, 1, 1,
                 'published', '{}', 'completed')",
    )
    .bind(activity)
    .bind(journey)
    .bind(subject)
    .execute(&mut *transaction)
    .await
    .expect("insert activity");
    sqlx::query("INSERT INTO tb_activity_objectives VALUES ($1, $2)")
        .bind(activity)
        .bind(objective)
        .execute(&mut *transaction)
        .await
        .expect("link objective");
    sqlx::query(
        "INSERT INTO tb_learning_sessions
             (id, journey_id, activity_id, subject_user_id, actor_identity_id, status, question_plan)
         VALUES ($1, $2, $3, $4, $4, 'finished', '{}')",
    )
    .bind(session)
    .bind(journey)
    .bind(activity)
    .bind(subject)
    .execute(&mut *transaction)
    .await
    .expect("insert session");
    sqlx::query(
        "INSERT INTO tb_attempts
             (id, learning_session_id, activity_id, subject_user_id, status, review_status, graded_at)
         VALUES ($1, $2, $3, $4, 'graded', 'not_required', now())",
    )
    .bind(attempt)
    .bind(session)
    .bind(activity)
    .bind(subject)
    .execute(&mut *transaction)
    .await
    .expect("insert attempt");
    sqlx::query(
        "INSERT INTO tb_mastery_evidence
             (id, subject_user_id, journey_id, objective_id, attempt_id, activity_id,
              evidence_type, value)
         VALUES ($1, $2, $3, $4, $5, $6, 'assessment', 0.8)",
    )
    .bind(evidence)
    .bind(subject)
    .bind(journey)
    .bind(objective)
    .bind(attempt)
    .bind(activity)
    .execute(&mut *transaction)
    .await
    .expect("insert evidence");
    transaction.commit().await.expect("commit fixture");
    (journey, objective, activity, evidence)
}
