use serde::Deserialize;

const BUILTIN_TEMPLATES: &str = include_str!("../templates/index.json");
const TOPIC_BLUEPRINTS: &str = include_str!("../templates/topic-blueprints.json");

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct TemplateCatalog {
    pub schema_version: u32,
    pub templates: Vec<LearningTemplate>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct LearningTemplate {
    pub id: String,
    pub version: u32,
    pub title: String,
    pub description: String,
    pub promise: String,
    pub when_to_use: String,
    pub inputs: TemplateInputs,
    pub stages: Vec<String>,
    pub activity_types: Vec<String>,
    pub output_contract: Vec<String>,
    pub quality_rules: Vec<String>,
    pub allowed_capabilities: Vec<String>,
    pub preview: TemplatePreview,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct TemplateInputs {
    pub required: Vec<String>,
    pub optional: Vec<String>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct TemplatePreview {
    pub shows: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct TemplateRecommendation {
    pub template_id: &'static str,
    pub reason: &'static str,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct TopicBlueprint {
    pub id: String,
    pub template_id: String,
    pub aliases: Vec<String>,
    pub objectives: Vec<TopicObjective>,
    pub first_activity: TopicFirstActivity,
    pub follow_up_activities: Vec<TopicActivity>,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct TopicObjective {
    pub verb: String,
    pub statement: String,
    pub success_criteria: String,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct TopicFirstActivity {
    pub kind: String,
    pub title: String,
    pub purpose: String,
    pub estimated_minutes: i64,
    pub context: String,
    pub instructions: String,
    pub questions: Vec<serde_json::Value>,
    pub objective_orders: Vec<i32>,
    pub status: String,
}

#[derive(Debug, Deserialize, Clone, PartialEq, Eq)]
pub struct TopicActivity {
    pub kind: String,
    pub title: String,
    pub objective_orders: Vec<i32>,
    pub payload: serde_json::Value,
    pub status: String,
}

pub fn builtin_catalog() -> Result<TemplateCatalog, String> {
    let catalog: TemplateCatalog = serde_json::from_str(BUILTIN_TEMPLATES)
        .map_err(|error| format!("invalid built-in learning templates: {error}"))?;
    validate_catalog(&catalog)?;
    Ok(catalog)
}

pub fn topic_blueprints() -> Result<Vec<TopicBlueprint>, String> {
    let topics: Vec<TopicBlueprint> = serde_json::from_str(TOPIC_BLUEPRINTS)
        .map_err(|error| format!("invalid topic blueprints: {error}"))?;
    if topics.iter().any(|topic| {
        topic.id.trim().is_empty()
            || topic.template_id.trim().is_empty()
            || topic.objectives.is_empty()
            || topic.first_activity.questions.is_empty()
            || topic.follow_up_activities.is_empty()
    }) {
        return Err("topic blueprints must contain identity, objectives, and questions".into());
    }
    Ok(topics)
}

pub fn find_topic_blueprint(prompt: &str) -> Result<Option<TopicBlueprint>, String> {
    let normalized = prompt.trim().to_lowercase();
    topic_blueprints().map(|topics| {
        topics.into_iter().find(|topic| {
            topic
                .aliases
                .iter()
                .any(|alias| normalized.contains(&alias.to_lowercase()))
        })
    })
}

pub fn find_template_blueprint(template_id: &str) -> Result<Option<TopicBlueprint>, String> {
    topic_blueprints().map(|topics| {
        topics
            .into_iter()
            .find(|topic| topic.template_id == template_id && topic.aliases.is_empty())
    })
}

/// Select a safe built-in starting shape before an agent enriches the journey.
///
/// This is intentionally conservative: it only recognizes unambiguous exam
/// and project language, and defaults all other intents to subject learning.
pub fn recommend_builtin_template(intent: &str) -> TemplateRecommendation {
    let normalized = intent.trim().to_lowercase();
    let has_signal = |signals: &[&str]| {
        normalized
            .split(|character: char| !character.is_alphanumeric())
            .any(|word| signals.contains(&word))
    };

    if has_signal(&[
        "exam",
        "certification",
        "certificate",
        "test",
        "qualification",
    ]) {
        return TemplateRecommendation {
            template_id: "exam-prep",
            reason: "The intent names an exam, certification, test, or qualification target.",
        };
    }

    if has_signal(&[
        "build", "create", "make", "project", "artifact", "app", "website",
    ]) {
        return TemplateRecommendation {
            template_id: "build-a-project",
            reason: "The intent names a project, artifact, or practical thing to make.",
        };
    }

    TemplateRecommendation {
        template_id: "learn-a-subject",
        reason: "The intent names a subject without an explicit exam or project target.",
    }
}

fn validate_catalog(catalog: &TemplateCatalog) -> Result<(), String> {
    if catalog.schema_version == 0 {
        return Err("template schema_version must be positive".into());
    }

    if catalog.templates.is_empty() {
        return Err("template catalog must not be empty".into());
    }

    for (index, template) in catalog.templates.iter().enumerate() {
        if template.id.trim().is_empty() {
            return Err(format!("template {index} has an empty id"));
        }
        if template.version == 0 {
            return Err(format!("template {} has an invalid version", template.id));
        }
        if template.title.trim().is_empty()
            || template.description.trim().is_empty()
            || template.promise.trim().is_empty()
        {
            return Err(format!(
                "template {} must have display metadata",
                template.id
            ));
        }
        if template.stages.is_empty()
            || template.activity_types.is_empty()
            || template.output_contract.is_empty()
            || template.quality_rules.is_empty()
            || template.allowed_capabilities.is_empty()
            || template.preview.shows.is_empty()
        {
            return Err(format!(
                "template {} is missing required contract fields",
                template.id
            ));
        }

        if catalog.templates[..index]
            .iter()
            .any(|previous| previous.id == template.id && previous.version == template.version)
        {
            return Err(format!(
                "template {} version {} is duplicated",
                template.id, template.version
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        TemplateRecommendation, builtin_catalog, find_topic_blueprint, recommend_builtin_template,
    };

    #[test]
    fn builtins_are_a_valid_versioned_catalog() {
        let catalog = builtin_catalog().expect("built-in catalog must parse");

        assert_eq!(catalog.schema_version, 1);
        assert_eq!(catalog.templates.len(), 3);
        assert!(
            catalog
                .templates
                .iter()
                .any(|template| template.id == "learn-a-subject")
        );
        assert!(
            catalog
                .templates
                .iter()
                .any(|template| template.id == "exam-prep")
        );
        assert!(
            catalog
                .templates
                .iter()
                .any(|template| template.id == "build-a-project")
        );
    }

    #[test]
    fn builtins_use_the_canonical_0_3_template_ids() {
        let catalog = builtin_catalog().expect("built-in catalog must parse");
        let ids: Vec<&str> = catalog
            .templates
            .iter()
            .map(|template| template.id.as_str())
            .collect();

        assert_eq!(
            ids,
            vec![
                "understand-a-subject",
                "prepare-for-an-exam",
                "build-a-project"
            ]
        );
    }

    #[test]
    fn topic_blueprints_are_valid_and_template_fallbacks_exist() {
        let topics = super::topic_blueprints().expect("topic blueprint catalog must parse");

        assert!(topics.iter().all(|topic| {
            !topic.id.is_empty()
                && !topic.template_id.is_empty()
                && !topic.objectives.is_empty()
                && !topic.first_activity.questions.is_empty()
                && !topic.follow_up_activities.is_empty()
        }));
        assert!(
            super::find_template_blueprint("learn-a-subject")
                .expect("topic blueprint catalog must parse")
                .is_some()
        );
        assert!(
            find_topic_blueprint("I want to learn an unlisted subject")
                .expect("topic blueprint catalog must parse")
                .is_none()
        );
    }

    #[test]
    fn recommendation_selects_exam_template_for_exam_intent() {
        assert_eq!(
            recommend_builtin_template("I want to prepare for a distributed systems exam"),
            TemplateRecommendation {
                template_id: "exam-prep",
                reason: "The intent names an exam, certification, test, or qualification target."
            }
        );
    }

    #[test]
    fn recommendation_selects_project_template_for_artifact_intent() {
        assert_eq!(
            recommend_builtin_template("I want to build an accessible website"),
            TemplateRecommendation {
                template_id: "build-a-project",
                reason: "The intent names a project, artifact, or practical thing to make."
            }
        );
    }

    #[test]
    fn recommendation_defaults_to_subject_template() {
        assert_eq!(
            recommend_builtin_template("I would like to learn distributed systems"),
            TemplateRecommendation {
                template_id: "learn-a-subject",
                reason: "The intent names a subject without an explicit exam or project target."
            }
        );
    }

    #[test]
    fn recommendation_does_not_match_signals_inside_other_words() {
        assert_eq!(
            recommend_builtin_template("I want to understand contest history"),
            TemplateRecommendation {
                template_id: "learn-a-subject",
                reason: "The intent names a subject without an explicit exam or project target."
            }
        );
    }
}
