use ame_api::{
    domain::note::{CreateNote, NoteError},
    note::NoteRepository,
    note_postgres::PgNoteRepository,
};
use sqlx::{migrate::Migrator, postgres::PgPoolOptions};
use uuid::Uuid;

static MIGRATOR: Migrator = sqlx::migrate!("../db/migrations");

#[tokio::test]
async fn private_note_resumes_at_its_original_content_version() {
    if std::env::var("AME_RUN_DB_TESTS").as_deref() != Ok("1") {
        eprintln!("skipping note integration test; set AME_RUN_DB_TESTS=1");
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
    let goal = Uuid::now_v7();
    let journey = Uuid::now_v7();
    let activity = Uuid::now_v7();
    for user in [subject, other] {
        let mut transaction = pool.begin().await.expect("begin identity");
        sqlx::query("SET CONSTRAINTS ALL DEFERRED")
            .execute(&mut *transaction)
            .await
            .unwrap();
        sqlx::query(
            "INSERT INTO tb_users (id, email, email_canonical, display_name)
             VALUES ($1, $2, $2, 'Note Learner')",
        )
        .bind(user)
        .bind(format!("note-{user}@example.test"))
        .execute(&mut *transaction)
        .await
        .unwrap();
        sqlx::query(
            "INSERT INTO tb_identities (id, identity_type, owner_user_id, label)
             VALUES ($1, 'human', $1, 'Note Learner')",
        )
        .bind(user)
        .execute(&mut *transaction)
        .await
        .unwrap();
        transaction.commit().await.unwrap();
    }
    sqlx::query(
        "INSERT INTO tb_learning_goals
            (id, subject_user_id, source_actor_id, raw_intent, normalized_statement)
         VALUES ($1, $2, $2, 'learn event time', 'Learn event time')",
    )
    .bind(goal)
    .bind(subject)
    .execute(&pool)
    .await
    .expect("seed goal");
    sqlx::query(
        "INSERT INTO tb_learning_journeys
            (id, goal_id, subject_user_id, source_actor_id, promise)
         VALUES ($1, $2, $3, $3, 'Understand event time')",
    )
    .bind(journey)
    .bind(goal)
    .bind(subject)
    .execute(&pool)
    .await
    .expect("seed journey");
    sqlx::query(
        "INSERT INTO tb_activities
            (id, journey_id, subject_user_id, source_actor_id, kind, title,
             order_index, content_version)
         VALUES ($1, $2, $3, $3, 'explanation', 'Event time', 0, 2)",
    )
    .bind(activity)
    .bind(journey)
    .bind(subject)
    .execute(&pool)
    .await
    .expect("seed study anchor");
    let repository = PgNoteRepository::new(pool.clone());
    let input = CreateNote {
        subject_user_id: subject,
        journey_id: journey,
        activity_id: Some(activity),
        content_version: Some(1),
        body: "Watermarks measure event-time progress.".into(),
        retry_key: "watermark-note-v1".into(),
    };
    let first = repository.create(input.clone()).await.expect("create note");
    assert_eq!(repository.create(input).await.unwrap(), first);
    assert!(
        repository
            .list(other, journey, Some(activity))
            .await
            .unwrap()
            .is_empty()
    );
    let edited = repository
        .update(
            subject,
            first.id,
            1,
            "Watermarks track event-time progress.".into(),
        )
        .await
        .expect("edit after return");
    assert_eq!(edited.content_version, Some(1));
    assert_eq!(edited.revision, 2);
    assert_eq!(
        repository.update(other, first.id, 2, "stolen".into()).await,
        Err(NoteError::NotFound)
    );
    repository
        .delete(subject, first.id)
        .await
        .expect("delete note");
    assert!(
        repository
            .list(subject, journey, Some(activity))
            .await
            .unwrap()
            .is_empty()
    );
}
