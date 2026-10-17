use std::sync::Arc;

use log::info;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::Mutex;

use crate::models::skill::{
    ExportedSkill, ImportConflictMode, Skill, SkillFrontmatter, SkillFull, SkillSummary,
    SlugValidation,
};
use crate::commands::settings::apply_settings_reload;
use crate::services::config::ConfigService;
use crate::services::placeholder_registry::{substitute, PlaceholderContext};
use crate::services::skill::{validate_slug, SkillError, SkillService};
use crate::services::skill_message;
use crate::services::sqlite_history::SqliteHistoryService;
use crate::Error;

#[derive(Serialize, Clone)]
struct SkillsChangedEvent {
    reason: String,
}

pub(crate) fn emit_changed(app: &AppHandle, reason: &str) -> crate::Result<()> {
    app.emit(
        "skills-changed",
        SkillsChangedEvent {
            reason: reason.to_string(),
        },
    )?;
    Ok(())
}

pub(crate) async fn rescan_and_prune(
    skill_service: &Mutex<SkillService>,
    config: &Mutex<ConfigService>,
    history: &Mutex<SqliteHistoryService>,
) -> crate::Result<()> {
    let order = config.lock().await.settings().skills_order.clone();
    let mut skill_guard = skill_service.lock().await;
    skill_guard.reload(&order)?;
    let history_guard = history.lock().await;
    skill_guard.prune_missing_skills(history_guard.conn())?;
    Ok(())
}

enum OrderPlacement<'a> {
    Keep,
    Append,
    After(&'a str),
}

impl OrderPlacement<'_> {
    fn apply(&self, order: &mut Vec<String>, slug: &str) {
        match self {
            Self::Keep => {}
            Self::Append => {
                if !order.iter().any(|n| n == slug) {
                    order.push(slug.to_string());
                }
            }
            Self::After(source) => match order.iter().position(|n| n == source) {
                Some(pos) => order.insert(pos + 1, slug.to_string()),
                None => order.push(slug.to_string()),
            },
        }
    }
}

async fn persist_skill(
    app: &AppHandle,
    slug: &str,
    placement: OrderPlacement<'_>,
    reason: &str,
) -> crate::Result<SkillFull> {
    let config = app.state::<Arc<Mutex<ConfigService>>>();
    let skill_service = app.state::<Arc<Mutex<SkillService>>>();
    let history = app.state::<Arc<Mutex<SqliteHistoryService>>>();

    rescan_and_prune(&skill_service, &config, &history).await?;
    if !matches!(placement, OrderPlacement::Keep) {
        let reloaded = config
            .lock()
            .await
            .mutate(|c| {
                let mut order = c.settings().skills_order.clone();
                placement.apply(&mut order, slug);
                c.update_skills_order(order);
            })?
            .reloaded;
        if reloaded {
            apply_settings_reload(app, reason).await?;
        }
    }
    emit_changed(app, reason)?;

    let skill_guard = skill_service.lock().await;
    let skill = skill_guard
        .get_skill(slug)
        .ok_or_else(|| Error::Other(format!("Skill '{slug}' missing after {reason}")))?;
    Ok(SkillFull::from(skill))
}

#[tauri::command]
pub async fn list_skills(
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
) -> crate::Result<Vec<SkillSummary>> {
    Ok(skill_service
        .lock()
        .await
        .list_skills()
        .iter()
        .map(SkillSummary::from)
        .collect())
}

#[tauri::command]
pub async fn list_skills_full(
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
) -> crate::Result<Vec<SkillFull>> {
    Ok(skill_service
        .lock()
        .await
        .list_skills()
        .iter()
        .map(SkillFull::from)
        .collect())
}

#[tauri::command]
pub async fn get_skill(
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    name: String,
) -> crate::Result<Skill> {
    skill_service
        .lock()
        .await
        .get_skill(&name)
        .cloned()
        .ok_or_else(|| Error::Other(format!("Skill not found: {name}")))
}

#[tauri::command]
pub async fn validate_skill_slug(
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    slug: String,
) -> crate::Result<SlugValidation> {
    if let Err(e) = validate_slug(&slug) {
        return Ok(SlugValidation {
            ok: false,
            error: Some(e.to_string()),
        });
    }
    let already_exists = skill_service
        .lock()
        .await
        .list_skills()
        .iter()
        .any(|s| s.name == slug);
    if already_exists {
        return Ok(SlugValidation {
            ok: false,
            error: Some(format!("Skill '{slug}' already exists")),
        });
    }
    Ok(SlugValidation {
        ok: true,
        error: None,
    })
}

#[tauri::command]
pub async fn create_skill(
    app: AppHandle,
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    slug: String,
    frontmatter: SkillFrontmatter,
    body: String,
) -> crate::Result<SkillFull> {
    if frontmatter.name != slug {
        return Err(Error::Other(
            "frontmatter.name must match slug".to_string(),
        ));
    }

    {
        let mut skill_guard = skill_service.lock().await;
        if skill_guard.get_skill(&slug).is_some() {
            return Err(Error::Other(format!("Skill '{slug}' already exists")));
        }
        skill_guard.write_skill(&slug, &frontmatter, &body)?;
    }

    persist_skill(&app, &slug, OrderPlacement::Append, "create").await
}

#[tauri::command]
pub async fn update_skill(
    app: AppHandle,
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    slug: String,
    frontmatter: SkillFrontmatter,
    body: String,
) -> crate::Result<SkillFull> {
    if frontmatter.name != slug {
        return Err(Error::Other(
            "frontmatter.name must match slug".to_string(),
        ));
    }
    {
        let mut skill_guard = skill_service.lock().await;
        if skill_guard.get_skill(&slug).is_none() {
            return Err(Error::Other(format!("Skill '{slug}' not found")));
        }
        skill_guard.write_skill(&slug, &frontmatter, &body)?;
    }

    persist_skill(&app, &slug, OrderPlacement::Keep, "update").await
}

#[tauri::command]
pub async fn delete_skill(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    history: State<'_, Arc<Mutex<SqliteHistoryService>>>,
    slug: String,
) -> crate::Result<()> {
    {
        let mut skill_guard = skill_service.lock().await;
        skill_guard.delete_skill_dir(&slug)?;
    }
    rescan_and_prune(&skill_service, &config, &history).await?;
    let reloaded = config
        .lock()
        .await
        .mutate(|c| {
            let mut order = c.settings().skills_order.clone();
            if order.iter().any(|n| n == &slug) {
                order.retain(|n| n != &slug);
                c.update_skills_order(order);
            }
        })?
        .reloaded;
    if reloaded {
        apply_settings_reload(&app, "delete").await?;
    }
    emit_changed(&app, "delete")?;
    Ok(())
}

#[tauri::command]
pub async fn duplicate_skill(
    app: AppHandle,
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    slug: String,
    new_slug: String,
) -> crate::Result<SkillFull> {
    let (frontmatter, body) = {
        let skill_guard = skill_service.lock().await;
        let source = skill_guard
            .get_skill(&slug)
            .ok_or_else(|| Error::Other(format!("Skill '{slug}' not found")))?;
        if skill_guard.get_skill(&new_slug).is_some() {
            return Err(Error::Other(format!("Skill '{new_slug}' already exists")));
        }
        let fm = duplicate_frontmatter(source, &new_slug);
        (fm, source.body.clone())
    };

    {
        let mut skill_guard = skill_service.lock().await;
        skill_guard.write_skill(&new_slug, &frontmatter, &body)?;
    }

    persist_skill(&app, &new_slug, OrderPlacement::After(&slug), "duplicate").await
}

#[tauri::command]
pub async fn reorder_skills(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    history: State<'_, Arc<Mutex<SqliteHistoryService>>>,
    order: Vec<String>,
) -> crate::Result<()> {
    let reloaded = config
        .lock()
        .await
        .mutate(|c| c.update_skills_order(order))?
        .reloaded;
    if reloaded {
        apply_settings_reload(&app, "reorder").await?;
    }
    rescan_and_prune(&skill_service, &config, &history).await?;
    emit_changed(&app, "reorder")?;
    Ok(())
}

#[tauri::command]
pub async fn import_skill_file(
    app: AppHandle,
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    content: String,
    on_conflict: Option<ImportConflictMode>,
) -> crate::Result<SkillFull> {
    let (frontmatter, body) = SkillService::parse_frontmatter(&content)?;
    validate_slug(&frontmatter.name)?;

    let mode = on_conflict.unwrap_or(ImportConflictMode::Reject);
    let mut target_slug = frontmatter.name.clone();
    {
        let skill_guard = skill_service.lock().await;
        if skill_guard.get_skill(&target_slug).is_some() {
            match mode {
                ImportConflictMode::Reject => {
                    return Err(Error::Skill(SkillError::AlreadyExists(target_slug)));
                }
                ImportConflictMode::Overwrite => {}
                ImportConflictMode::RenameSuffix => {
                    let mut counter = 1;
                    let candidate = loop {
                        let candidate = format!("{}-imported-{counter}", frontmatter.name);
                        if skill_guard.get_skill(&candidate).is_none() {
                            break candidate;
                        }
                        counter += 1;
                        if counter > 99 {
                            return Err(Error::Other(format!(
                                "Could not find unique slug after 99 attempts for {target_slug}"
                            )));
                        }
                    };
                    target_slug = candidate;
                }
            }
        }
    }

    let mut fm_to_write = frontmatter.clone();
    fm_to_write.name = target_slug.clone();

    {
        let mut skill_guard = skill_service.lock().await;
        skill_guard.write_skill(&target_slug, &fm_to_write, &body)?;
    }

    persist_skill(&app, &target_slug, OrderPlacement::Append, "import").await
}

fn duplicate_frontmatter(source: &Skill, new_slug: &str) -> SkillFrontmatter {
    SkillFrontmatter {
        name: new_slug.to_string(),
        display_name: Some(format!("{} (copy)", source.display_name)),
        description: source.description.clone(),
        model: source.model.clone(),
        parameters: source.parameters.clone(),
        offered_in: source.offered_in.clone(),
    }
}

#[tauri::command]
pub async fn export_skill(
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    slug: String,
) -> crate::Result<ExportedSkill> {
    let guard = skill_service.lock().await;
    if guard.get_skill(&slug).is_none() {
        return Err(Error::Other(format!("Skill '{slug}' not found")));
    }
    let content = guard.skill_file_content(&slug)?;
    Ok(ExportedSkill {
        filename: format!("{slug}.md"),
        content,
    })
}

#[tauri::command]
pub async fn preview_skill_message(
    body: String,
    sample_input: String,
    skill_name: Option<String>,
) -> crate::Result<String> {
    let name = skill_name.unwrap_or_else(|| "preview".to_string());
    let preview_ctx = PlaceholderContext::with_apps("(preview)", "(preview)");
    let rendered_body = substitute(&body, &preview_ctx);
    info!("composing skill preview for '{name}'");
    Ok(skill_message::wrap_skill(&name, &rendered_body, &sample_input))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::skill::OfferedIn;

    #[test]
    fn duplicate_frontmatter_keeps_offered_in() {
        let source = Skill {
            name: "src".into(),
            display_name: "Src".into(),
            description: None,
            model: None,
            parameters: None,
            offered_in: Some(vec![OfferedIn::Pill]),
            body: String::new(),
            file_path: Default::default(),
            skill_version_id: None,
        };
        let fm = duplicate_frontmatter(&source, "copy");
        assert_eq!(fm.offered_in, Some(vec![OfferedIn::Pill]));
        assert_eq!(fm.name, "copy");
    }
}
