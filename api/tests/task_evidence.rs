use ame_api::domain::{
    progress::{EvidenceSource, MasteryEvidenceInput},
    task::{TaskEvaluationMethod, TaskReviewStatus, TaskSubmissionStatus},
};
use ame_api::progress::ProgressRepository;
use ame_api::progress_postgres::PgProgressRepository;
use ame_api::task::{
    ReviewTaskSubmission, StartTaskSubmission, TaskReviewOutcome, TaskSubmissionRepository,
};
use ame_api::task_postgres::PgTaskSubmissionRepository;
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
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

    let tasks = PgTaskSubmissionRepository::new(pool.clone());
    let started = tasks
        .start(StartTaskSubmission {
            subject_user_id: subject,
            task_id: activity,
            content_version: 2,
            response: serde_json::json!({"optionId": "event-time"}),
            evaluation_method: TaskEvaluationMethod::Agent,
        })
        .await
        .expect("start task submission");
    let submitted = tasks
        .submit(subject, started.id)
        .await
        .expect("submit task");
    assert_eq!(submitted.envelope.status, TaskSubmissionStatus::Submitted);
    assert_eq!(submitted.envelope.review_status, TaskReviewStatus::Pending);
    let reviewed = tasks
        .review(ReviewTaskSubmission {
            reviewer_user_id: subject,
            submission_id: submitted.id,
            outcome: TaskReviewOutcome::Reviewed,
            score: Some(0.9),
            feedback: Some(serde_json::json!({"note": "Correct event-time reasoning"})),
        })
        .await
        .expect("review task submission");
    let context = tasks
        .evidence_context(reviewed.id)
        .await
        .expect("resolve evidence context");
    let progress = PgProgressRepository::new(pool);
    let evidence = progress
        .record_evidence(MasteryEvidenceInput {
            subject_user_id: context.subject_user_id,
            journey_id: context.journey_id,
            objective_id: context.objective_ids[0],
            activity_id: context.activity_id,
            source: EvidenceSource::Task {
                submission_id: reviewed.id,
            },
            content_version: context.content_version,
            value: context.score,
            derivation_version: 1,
        })
        .await
        .expect("record task evidence");
    assert_eq!(
        evidence.input.source,
        EvidenceSource::Task {
            submission_id: reviewed.id
        }
    );
    let snapshot = progress
        .snapshot(subject, journey, objective)
        .await
        .expect("calculate mastery snapshot");
    assert_eq!(snapshot.mastery, 0.9);
    assert_eq!(snapshot.evidence_count, 1);
}
