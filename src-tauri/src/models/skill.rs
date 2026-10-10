use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OfferedIn {
    ContextMenu,
    Pill,
}

pub fn is_offered_in(offered_in: &Option<Vec<OfferedIn>>, place: OfferedIn) -> bool {
    offered_in
        .as_ref()
        .is_none_or(|places| places.contains(&place))
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SkillFrontmatter {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offered_in: Option<Vec<OfferedIn>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub model: Option<String>,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    pub offered_in: Option<Vec<OfferedIn>>,
    pub body: String,
    #[serde(skip)]
    pub file_path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill_version_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillSummary {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub offered_in: Option<Vec<OfferedIn>>,
}

impl From<&Skill> for SkillSummary {
    fn from(skill: &Skill) -> Self {
        Self {
            name: skill.name.clone(),
            display_name: skill.display_name.clone(),
            description: skill.description.clone(),
            offered_in: skill.offered_in.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillFull {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub model: Option<String>,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    pub offered_in: Option<Vec<OfferedIn>>,
    pub body: String,
    pub file_path: String,
}

impl From<&Skill> for SkillFull {
    fn from(skill: &Skill) -> Self {
        Self {
            name: skill.name.clone(),
            display_name: skill.display_name.clone(),
            description: skill.description.clone(),
            model: skill.model.clone(),
            parameters: skill.parameters.clone(),
            offered_in: skill.offered_in.clone(),
            body: skill.body.clone(),
            file_path: skill.file_path.to_string_lossy().to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SlugValidation {
    pub ok: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImportConflictMode {
    Reject,
    RenameSuffix,
    Overwrite,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportedSkill {
    pub filename: String,
    pub content: String,
}
