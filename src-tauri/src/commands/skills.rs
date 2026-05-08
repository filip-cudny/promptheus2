use std::sync::Arc;

use log::info;
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::models::skill::{
    ExportedSkill, ImportConflictMode, Skill, SkillFrontmatter, SkillFull, SkillSummary,
    SlugValidation,
};
use crate::services::config::ConfigService;
use crate::services::execution::substitute_environment_placeholders;
use crate::services::skill::{validate_slug, SkillError, SkillService};
use crate::services::sqlite_history::SqliteHistoryService;
use crate::Error;

#[derive(Serialize, Clone)]
struct SkillsChangedEvent {
    reason: String,
}

fn emit_changed(app: &AppHandle, reason: &str) -> crate::Result<()> {
    app.emit(
        "skills-changed",
        SkillsChangedEvent {
            reason: reason.to_string(),
        },
    )?;
    Ok(())
}

async fn rescan_and_prune(
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
pub async fn get_skill_body(
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    name: String,
) -> crate::Result<String> {
    skill_service
        .lock()
        .await
        .get_skill(&name)
        .map(|s| s.body.clone())
        .ok_or_else(|| Error::Other(format!("Skill not found: {name}")))
}

#[tauri::command]
pub async fn reload_skills(
    config: State<'_, Arc<Mutex<ConfigService>>>,
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    history: State<'_, Arc<Mutex<SqliteHistoryService>>>,
) -> crate::Result<()> {
    rescan_and_prune(&skill_service, &config, &history).await
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
    config: State<'_, Arc<Mutex<ConfigService>>>,
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    history: State<'_, Arc<Mutex<SqliteHistoryService>>>,
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

    rescan_and_prune(&skill_service, &config, &history).await?;
    {
        let mut config_guard = config.lock().await;
        let mut order = config_guard.settings().skills_order.clone();
        if !order.contains(&slug) {
            order.push(slug.clone());
            config_guard.update_skills_order(order);
            config_guard.save()?;
        }
    }
    emit_changed(&app, "create")?;

    let skill_guard = skill_service.lock().await;
    let skill = skill_guard
        .get_skill(&slug)
        .ok_or_else(|| Error::Other(format!("Skill '{slug}' missing after create")))?;
    Ok(SkillFull::from(skill))
}

#[tauri::command]
pub async fn update_skill(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    history: State<'_, Arc<Mutex<SqliteHistoryService>>>,
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

    rescan_and_prune(&skill_service, &config, &history).await?;
    emit_changed(&app, "update")?;

    let skill_guard = skill_service.lock().await;
    let skill = skill_guard
        .get_skill(&slug)
        .ok_or_else(|| Error::Other(format!("Skill '{slug}' missing after update")))?;
    Ok(SkillFull::from(skill))
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
    {
        let mut config_guard = config.lock().await;
        let mut order = config_guard.settings().skills_order.clone();
        if order.iter().any(|n| n == &slug) {
            order.retain(|n| n != &slug);
            config_guard.update_skills_order(order);
            config_guard.save()?;
        }
    }
    emit_changed(&app, "delete")?;
    Ok(())
}

#[tauri::command]
pub async fn duplicate_skill(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    history: State<'_, Arc<Mutex<SqliteHistoryService>>>,
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
        let display_suffix = format!("{} (copy)", source.display_name);
        let fm = SkillFrontmatter {
            name: new_slug.clone(),
            display_name: Some(display_suffix),
            description: source.description.clone(),
            model: source.model.clone(),
            parameters: source.parameters.clone(),
        };
        (fm, source.body.clone())
    };

    {
        let mut skill_guard = skill_service.lock().await;
        skill_guard.write_skill(&new_slug, &frontmatter, &body)?;
    }
    rescan_and_prune(&skill_service, &config, &history).await?;
    {
        let mut config_guard = config.lock().await;
        let mut order = config_guard.settings().skills_order.clone();
        if let Some(pos) = order.iter().position(|n| n == &slug) {
            order.insert(pos + 1, new_slug.clone());
        } else {
            order.push(new_slug.clone());
        }
        config_guard.update_skills_order(order);
        config_guard.save()?;
    }
    emit_changed(&app, "duplicate")?;

    let skill_guard = skill_service.lock().await;
    let skill = skill_guard
        .get_skill(&new_slug)
        .ok_or_else(|| Error::Other(format!("Skill '{new_slug}' missing after duplicate")))?;
    Ok(SkillFull::from(skill))
}

#[tauri::command]
pub async fn reorder_skills(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    history: State<'_, Arc<Mutex<SqliteHistoryService>>>,
    order: Vec<String>,
) -> crate::Result<()> {
    {
        let mut config_guard = config.lock().await;
        config_guard.update_skills_order(order);
        config_guard.save()?;
    }
    rescan_and_prune(&skill_service, &config, &history).await?;
    emit_changed(&app, "reorder")?;
    Ok(())
}

#[tauri::command]
pub async fn import_skill_file(
    app: AppHandle,
    config: State<'_, Arc<Mutex<ConfigService>>>,
    skill_service: State<'_, Arc<Mutex<SkillService>>>,
    history: State<'_, Arc<Mutex<SqliteHistoryService>>>,
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
    rescan_and_prune(&skill_service, &config, &history).await?;
    {
        let mut config_guard = config.lock().await;
        let mut order = config_guard.settings().skills_order.clone();
        if !order.contains(&target_slug) {
            order.push(target_slug.clone());
            config_guard.update_skills_order(order);
            config_guard.save()?;
        }
    }
    emit_changed(&app, "import")?;

    let skill_guard = skill_service.lock().await;
    let skill = skill_guard
        .get_skill(&target_slug)
        .ok_or_else(|| Error::Other(format!("Skill '{target_slug}' missing after import")))?;
    Ok(SkillFull::from(skill))
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
    let rendered_body =
        substitute_environment_placeholders(&body, "(preview)", "(preview)");
    info!("composing skill preview for '{name}'");
    Ok(format!(
        "<skill name=\"{name}\">\n{rendered_body}\n</skill>\n\n<input>\n{sample_input}\n</input>"
    ))
}
