//! Quiz planning.

use std::str::FromStr;

use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::domain::error::{ApiError, FieldError};
use crate::domain::question::{McPayload, QuestionKind};
use crate::domain::session::{PlanItem, QuestionPlan};

pub const DEFAULT_QUIZ_COUNT: usize = 10;
pub const MAX_QUIZ_COUNT: usize = 100;
pub const DEFAULT_EXCLUDE_RECENT_HOURS: i64 = 24;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TagsMode {
    #[default]
    Any,
    All,
}

impl TagsMode {
    pub const fn as_str(self) -> &'static str {
        match self {
            TagsMode::Any => "any",
            TagsMode::All => "all",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct QuizPlanRequest {
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub tags_mode: TagsMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty_min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub difficulty_max: Option<f64>,
    #[serde(default = "default_quiz_count")]
    pub count: usize,
    #[serde(default = "default_exclude_recent_hours")]
    pub exclude_recent_hours: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct PlanWarning {
    pub requested: usize,
    pub planned: usize,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, ToSchema)]
pub struct QuizPlan {
    pub question_plan: QuestionPlan,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<PlanWarning>,
}

#[derive(Debug, Clone)]
struct PlanCandidate {
    question_id: Uuid,
    kind: QuestionKind,
    version: i32,
    payload: serde_json::Value,
}

fn default_quiz_count() -> usize {
    DEFAULT_QUIZ_COUNT
}

fn default_exclude_recent_hours() -> i64 {
    DEFAULT_EXCLUDE_RECENT_HOURS
}

pub async fn plan_quiz(
    pool: &PgPool,
    user_id: Uuid,
    request: &QuizPlanRequest,
) -> Result<QuizPlan, ApiError> {
    validate_request(request)?;

    let tags = normalize_tags(&request.tags);
    let mode = request.tags_mode.as_str().to_string();
    let rows = sqlx::query(
        "SELECT q.id, q.kind, q.version, q.payload \
         FROM questions q \
         WHERE q.status = 'live' \
           AND ($2::double precision IS NULL OR q.rating >= $2) \
           AND ($3::double precision IS NULL OR q.rating <= $3) \
           AND (cardinality($4::text[]) = 0 \
                OR ($5::text = 'any' AND EXISTS ( \
                    SELECT 1 FROM question_tags qt \
                    JOIN tags t ON t.id = qt.tag_id \
                    WHERE qt.question_id = q.id AND t.name = ANY($4::text[]) \
                )) \
                OR ($5::text = 'all' AND NOT EXISTS ( \
                    SELECT 1 FROM unnest($4::text[]) required(name) \
                    WHERE NOT EXISTS ( \
                        SELECT 1 FROM question_tags qt \
                        JOIN tags t ON t.id = qt.tag_id \
                        WHERE qt.question_id = q.id AND t.name = required.name \
                    ) \
                ))) \
           AND ($6::bigint IS NULL OR NOT EXISTS ( \
                SELECT 1 FROM attempts a \
                WHERE a.user_id = $1 \
                  AND a.question_id = q.id \
                  AND a.created_at >= now() - ($6::bigint * interval '1 hour') \
           ))",
    )
    .bind(user_id)
    .bind(request.difficulty_min)
    .bind(request.difficulty_max)
    .bind(&tags)
    .bind(mode)
    .bind(if request.exclude_recent_hours == 0 {
        None
    } else {
        Some(request.exclude_recent_hours)
    })
    .fetch_all(pool)
    .await
    .map_err(internal)?;

    let candidates = rows
        .into_iter()
        .map(|row| {
            let kind_str: String = row.get("kind");
            let kind = QuestionKind::from_str(&kind_str)
                .map_err(|e| ApiError::Internal(anyhow::anyhow!("invalid kind in DB: {e}")))?;
            Ok(PlanCandidate {
                question_id: row.get("id"),
                kind,
                version: row.get("version"),
                payload: row.get("payload"),
            })
        })
        .collect::<Result<Vec<_>, ApiError>>()?;

    let mut rng = rand::thread_rng();
    plan_from_candidates(candidates, request.count, &mut rng)
}

fn plan_from_candidates<R: rand::Rng + ?Sized>(
    mut candidates: Vec<PlanCandidate>,
    requested_count: usize,
    rng: &mut R,
) -> Result<QuizPlan, ApiError> {
    candidates.shuffle(rng);
    candidates.truncate(requested_count);

    let items = candidates
        .iter()
        .map(plan_item_from_candidate)
        .collect::<Result<Vec<_>, ApiError>>()?;
    let planned = items.len();

    Ok(QuizPlan {
        question_plan: QuestionPlan { items },
        warning: (planned < requested_count).then(|| PlanWarning {
            requested: requested_count,
            planned,
            reason: "pool_short".to_string(),
        }),
    })
}

fn plan_item_from_candidate(candidate: &PlanCandidate) -> Result<PlanItem, ApiError> {
    let option_order = if candidate.kind == QuestionKind::Mc {
        let payload: McPayload =
            serde_json::from_value(candidate.payload.clone()).map_err(|e| {
                ApiError::InvalidPayload {
                    kind: candidate.kind.as_str().to_string(),
                    reason: e.to_string(),
                }
            })?;
        let mut order: Vec<usize> = (0..payload.options.len()).collect();
        order.shuffle(&mut rand::thread_rng());
        Some(order)
    } else {
        None
    };

    Ok(PlanItem {
        question_id: candidate.question_id,
        version: candidate.version,
        section: None,
        option_order,
    })
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut normalized = tags
        .iter()
        .map(|tag| tag.trim().to_ascii_lowercase())
        .filter(|tag| !tag.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

fn validate_request(request: &QuizPlanRequest) -> Result<(), ApiError> {
    let mut fields = Vec::new();
    if request.count == 0 || request.count > MAX_QUIZ_COUNT {
        fields.push(FieldError {
            field: "count".to_string(),
            message: format!("count must be between 1 and {MAX_QUIZ_COUNT}"),
        });
    }
    if request.exclude_recent_hours < 0 {
        fields.push(FieldError {
            field: "exclude_recent_hours".to_string(),
            message: "exclude_recent_hours must be >= 0".to_string(),
        });
    }
    if matches!(
        (request.difficulty_min, request.difficulty_max),
        (Some(min), Some(max)) if min > max
    ) {
        fields.push(FieldError {
            field: "difficulty".to_string(),
            message: "difficulty_min must be <= difficulty_max".to_string(),
        });
    }

    if fields.is_empty() {
        Ok(())
    } else {
        Err(ApiError::Validation(fields))
    }
}

fn internal<E: Into<anyhow::Error>>(e: E) -> ApiError {
    ApiError::Internal(e.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{SeedableRng, rngs::StdRng};

    fn mc_candidate(question_id: Uuid) -> PlanCandidate {
        PlanCandidate {
            question_id,
            kind: QuestionKind::Mc,
            version: 2,
            payload: serde_json::json!({
                "options": ["a", "b", "c"],
                "correct_index": 1
            }),
        }
    }

    #[test]
    fn normalize_tags_lowercases_dedupes_and_discards_blank_values() {
        assert_eq!(
            normalize_tags(&[
                " Rust ".to_string(),
                "rust".to_string(),
                "".to_string(),
                "ASYNC".to_string()
            ]),
            vec!["async".to_string(), "rust".to_string()]
        );
    }

    #[test]
    fn plan_from_candidates_samples_without_replacement_and_warns_when_short() {
        let q1 = Uuid::now_v7();
        let q2 = Uuid::now_v7();
        let candidates = vec![mc_candidate(q1), mc_candidate(q2)];
        let mut rng = StdRng::seed_from_u64(1);

        let plan = plan_from_candidates(candidates, 3, &mut rng).unwrap();

        assert_eq!(plan.question_plan.items.len(), 2);
        assert_eq!(
            plan.warning,
            Some(PlanWarning {
                requested: 3,
                planned: 2,
                reason: "pool_short".to_string()
            })
        );
        assert_ne!(
            plan.question_plan.items[0].question_id,
            plan.question_plan.items[1].question_id
        );
        assert!(
            plan.question_plan.items[0]
                .option_order
                .as_ref()
                .is_some_and(|order| order.len() == 3)
        );
    }

    #[test]
    fn validate_rejects_empty_count() {
        let err = validate_request(&QuizPlanRequest {
            count: 0,
            ..QuizPlanRequest {
                tags: Vec::new(),
                tags_mode: TagsMode::Any,
                difficulty_min: None,
                difficulty_max: None,
                count: DEFAULT_QUIZ_COUNT,
                exclude_recent_hours: DEFAULT_EXCLUDE_RECENT_HOURS,
            }
        })
        .unwrap_err();

        match err {
            ApiError::Validation(fields) => assert_eq!(fields[0].field, "count"),
            other => panic!("expected validation error, got {other:?}"),
        }
    }
}
