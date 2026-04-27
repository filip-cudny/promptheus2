use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
pub struct SkillFrontmatter {
    pub name: String,
    pub description: Option<String>,
    pub display_name: Option<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub parameters: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Skill {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
    pub model: Option<String>,
    pub parameters: Option<HashMap<String, serde_json::Value>>,
    pub body: String,
    #[serde(skip)]
    pub file_path: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillSummary {
    pub name: String,
    pub display_name: String,
    pub description: Option<String>,
}

impl From<&Skill> for SkillSummary {
    fn from(skill: &Skill) -> Self {
        Self {
            name: skill.name.clone(),
            display_name: skill.display_name.clone(),
            description: skill.description.clone(),
        }
    }
}
