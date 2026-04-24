use std::path::PathBuf;

const EXPECTED_CAPABILITY_FILES: &[&str] = &["default.json"];

fn main() {
    let capabilities_dir = PathBuf::from("capabilities");
    println!("cargo:rerun-if-changed=capabilities");

    let mut files: Vec<String> = std::fs::read_dir(&capabilities_dir)
        .expect("read capabilities/ dir")
        .filter_map(|e| {
            let path = e.ok()?.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                path.file_name().and_then(|n| n.to_str()).map(String::from)
            } else {
                None
            }
        })
        .collect();
    files.sort();

    if files != EXPECTED_CAPABILITY_FILES {
        panic!(
            "capabilities/ contents changed: found {files:?}, expected {EXPECTED_CAPABILITY_FILES:?}.\n\
             Every capability file must be audited by lib.rs::assert_no_ai_webview_ipc_grant.\n\
             If you added or removed a file, update CAPABILITY_FILES in lib.rs AND \
             EXPECTED_CAPABILITY_FILES in build.rs."
        );
    }

    tauri_build::build()
}
