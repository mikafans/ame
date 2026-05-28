//! Micro-benchmarks for per-request hot paths.
//!
//! These guard against regressions in the cheap, frequently-called pure
//! functions: token verification (runs on every authenticated request),
//! response grading, and the Elo rating update. Run with `make bench`.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use uuid::Uuid;

use ame_api::auth::token::{generate_secret, hash_secret, verify_token_secret};
use ame_api::domain::attempt::{AttemptPresentation, AttemptResponse};
use ame_api::domain::question::QuestionKind;
use ame_api::engine::elo::{QuestionRating, UserTagRating, update_elo};
use ame_api::engine::graders::grade_response;

fn bench_token_verify(c: &mut Criterion) {
    let secret = generate_secret();
    let hash = hash_secret(&secret);

    let mut group = c.benchmark_group("token_verify");
    group.bench_function("match", |b| {
        b.iter(|| verify_token_secret(black_box(&hash), black_box(&secret)))
    });
    group.bench_function("mismatch", |b| {
        b.iter(|| verify_token_secret(black_box(&hash), black_box("wrong-secret")))
    });
    group.finish();
}

fn bench_grade_mc(c: &mut Criterion) {
    let payload = serde_json::json!({
        "options": ["Venus", "Mercury", "Earth", "Mars"],
        "correct_index": 1,
    });
    let response = AttemptResponse::Mc {
        selected_position: 1,
    };
    let presentation = AttemptPresentation {
        option_order: Some(vec![0, 1, 2, 3]),
    };

    c.bench_function("grade_response_mc", |b| {
        b.iter(|| {
            grade_response(
                black_box(QuestionKind::Mc),
                black_box(&payload),
                black_box(&response),
                black_box(&presentation),
                black_box(1),
            )
        })
    });
}

fn bench_update_elo(c: &mut Criterion) {
    let question = QuestionRating {
        rating: 1200.0,
        attempts_count: 10,
    };
    let user_tags = vec![
        UserTagRating {
            tag_id: Uuid::now_v7(),
            rating: 1180.0,
            attempts_count: 5,
        },
        UserTagRating {
            tag_id: Uuid::now_v7(),
            rating: 1250.0,
            attempts_count: 40,
        },
    ];

    c.bench_function("update_elo", |b| {
        b.iter(|| {
            update_elo(
                black_box(question.clone()),
                black_box(&user_tags),
                black_box(1.0),
            )
        })
    });
}

criterion_group!(
    benches,
    bench_token_verify,
    bench_grade_mc,
    bench_update_elo
);
criterion_main!(benches);
