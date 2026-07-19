use serde::Deserialize;

const BUILTIN_TEMPLATES: &str = include_str!("../templates/index.json");

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

pub fn builtin_catalog() -> Result<TemplateCatalog, String> {
    let catalog: TemplateCatalog = serde_json::from_str(BUILTIN_TEMPLATES)
        .map_err(|error| format!("invalid built-in learning templates: {error}"))?;
    validate_catalog(&catalog)?;
    Ok(catalog)
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
        if template.title.trim().is_empty() || template.description.trim().is_empty() {
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
    use super::{TemplateRecommendation, builtin_catalog, recommend_builtin_template};

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
    fn recommendation_selects_exam_template_for_exam_intent() {
        assert_eq!(
            recommend_builtin_template("I want to prepare for a music theory exam"),
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
            recommend_builtin_template("I would like to learn music theory"),
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
