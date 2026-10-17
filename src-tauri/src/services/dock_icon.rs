use crate::models::settings::Skin;

const LOGO_SVG: &[u8] = include_bytes!("../../icons/logo.svg");
const LOGO_MONO_SVG: &[u8] = include_bytes!("../../icons/logo-mono.svg");

fn logo_svg(logo: Skin) -> &'static [u8] {
    match logo {
        Skin::Color => LOGO_SVG,
        Skin::Mono => LOGO_MONO_SVG,
    }
}

#[cfg(target_os = "linux")]
pub fn apply(app: &tauri::AppHandle, logo: Skin) {
    use tauri::Manager;

    let data_dir = match app.path().data_dir() {
        Ok(dir) => dir,
        Err(e) => {
            log::warn!(target: "app_lib::services::dock_icon", "data dir unavailable: {e}");
            return;
        }
    };
    match write_override(&data_dir, logo_svg(logo)) {
        Ok(true) => log::info!(
            target: "app_lib::services::dock_icon",
            "dock icon set to {logo:?} in {}",
            data_dir.display(),
        ),
        Ok(false) => {}
        Err(e) => log::warn!(
            target: "app_lib::services::dock_icon",
            "writing dock icon override failed: {e}",
        ),
    }
}

#[cfg(target_os = "linux")]
fn write_override(data_dir: &std::path::Path, svg: &[u8]) -> std::io::Result<bool> {
    const SIZE_DIRS: [&str; 4] = ["scalable", "32x32", "128x128", "256x256@2"];

    let hicolor = data_dir.join("icons/hicolor");
    let mut written = false;
    for size_dir in SIZE_DIRS {
        let apps = hicolor.join(size_dir).join("apps");
        let file = apps.join("promptheus.svg");
        if std::fs::read(&file).is_ok_and(|current| current == svg) {
            continue;
        }
        std::fs::create_dir_all(&apps)?;
        std::fs::write(&file, svg)?;
        written = true;
    }
    if written {
        std::fs::File::open(&hicolor)?.set_modified(std::time::SystemTime::now())?;
    }
    Ok(written)
}

#[cfg(target_os = "macos")]
static REMEMBERED: std::sync::Mutex<Option<Skin>> = std::sync::Mutex::new(None);

#[cfg(target_os = "macos")]
pub fn apply(app: &tauri::AppHandle, logo: Skin) {
    *REMEMBERED.lock().unwrap() = Some(logo);
    if let Err(e) = app.run_on_main_thread(move || set_application_icon(logo)) {
        log::warn!(
            target: "app_lib::services::dock_icon",
            "run_on_main_thread for the dock icon failed: {e}",
        );
    }
}

#[cfg(target_os = "macos")]
pub fn reapply_remembered(app: &tauri::AppHandle) {
    let remembered = *REMEMBERED.lock().unwrap();
    if let Some(logo) = remembered {
        apply(app, logo);
    }
}

#[cfg(target_os = "macos")]
fn set_application_icon(logo: Skin) {
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;

    let Some(mtm) = MainThreadMarker::new() else {
        log::warn!(
            target: "app_lib::services::dock_icon",
            "dock icon skipped: not on the main thread",
        );
        return;
    };
    let ns_app = NSApplication::sharedApplication(mtm);
    match logo {
        Skin::Color => unsafe { ns_app.setApplicationIconImage(None) },
        Skin::Mono => {
            let Some(image) = svg_image(logo_svg(logo)) else {
                log::warn!(
                    target: "app_lib::services::dock_icon",
                    "SVG logo was not decoded into an NSImage; dock icon unchanged",
                );
                return;
            };
            unsafe { ns_app.setApplicationIconImage(Some(&image)) }
        }
    }
    log::info!(target: "app_lib::services::dock_icon", "dock icon set to {logo:?}");
}

#[cfg(target_os = "macos")]
fn svg_image(svg: &[u8]) -> Option<objc2::rc::Retained<objc2_app_kit::NSImage>> {
    use std::ffi::c_void;

    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;
    use objc2::{class, msg_send, AnyThread};
    use objc2_app_kit::NSImage;

    unsafe {
        let data: Retained<AnyObject> =
            msg_send![class!(NSData), dataWithBytes: svg.as_ptr().cast::<c_void>(), length: svg.len()];
        msg_send![NSImage::alloc(), initWithData: &*data]
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use std::fs::{self, File};
    use std::path::Path;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::*;

    const SIZE_DIRS: [&str; 4] = ["scalable", "32x32", "128x128", "256x256@2"];

    fn old_mtime() -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(1000)
    }

    fn hicolor_mtime(data_dir: &Path) -> SystemTime {
        fs::metadata(data_dir.join("icons/hicolor"))
            .unwrap()
            .modified()
            .unwrap()
    }

    fn reset_mtime(data_dir: &Path) {
        File::open(data_dir.join("icons/hicolor"))
            .unwrap()
            .set_modified(old_mtime())
            .unwrap();
    }

    fn assert_files_hold(data_dir: &Path, svg: &[u8]) {
        for size_dir in SIZE_DIRS {
            let file = data_dir
                .join("icons/hicolor")
                .join(size_dir)
                .join("apps/promptheus.svg");
            assert_eq!(fs::read(file).unwrap(), svg, "{size_dir}");
        }
    }

    #[test]
    fn override_follows_the_skin_and_bumps_mtime_only_on_change() {
        let tmp = tempfile::tempdir().unwrap();
        fs::create_dir_all(tmp.path().join("icons/hicolor")).unwrap();
        reset_mtime(tmp.path());

        assert!(write_override(tmp.path(), logo_svg(Skin::Mono)).unwrap());
        assert_files_hold(tmp.path(), LOGO_MONO_SVG);
        assert_ne!(hicolor_mtime(tmp.path()), old_mtime());

        reset_mtime(tmp.path());
        assert!(!write_override(tmp.path(), logo_svg(Skin::Mono)).unwrap());
        assert_eq!(hicolor_mtime(tmp.path()), old_mtime());

        assert!(write_override(tmp.path(), logo_svg(Skin::Color)).unwrap());
        assert_files_hold(tmp.path(), LOGO_SVG);
        assert_ne!(hicolor_mtime(tmp.path()), old_mtime());
    }
}
