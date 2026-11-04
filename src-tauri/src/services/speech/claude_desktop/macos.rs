use std::cell::RefCell;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{define_class, msg_send, AnyThread, DefinedClass};
use objc2_app_kit::{
    NSPasteboard, NSPasteboardContentsOptions, NSPasteboardItem, NSPasteboardItemDataProvider,
    NSPasteboardType, NSPasteboardTypeFileURL, NSPasteboardWriting, NSWorkspace,
};
use objc2_core_graphics::{
    CGEvent, CGEventFlags, CGEventSource, CGEventSourceStateID, CGEventTapLocation, CGKeyCode,
};
use objc2_foundation::{NSArray, NSData, NSObject, NSObjectProtocol, NSString, NSURL};
use tauri::AppHandle;
use tokio::sync::{mpsc, oneshot};

use super::{Outcome, LOG_TARGET};

const CLAUDE_BUNDLE: &str = "com.anthropic.claudefordesktop";
const FRONTMOST_POLL: Duration = Duration::from_millis(100);
const FRONTMOST_TIMEOUT: Duration = Duration::from_secs(20);
const SETTLE_WARM: Duration = Duration::from_millis(600);
const SETTLE_COLD: Duration = Duration::from_millis(4000);
const READ_TIMEOUT: Duration = Duration::from_secs(3);
const RESTORE_DELAY: Duration = Duration::from_millis(500);
const KEY_V: CGKeyCode = 9;
const KEY_RETURN: CGKeyCode = 36;

#[link(name = "ApplicationServices", kind = "framework")]
extern "C" {
    fn AXIsProcessTrusted() -> bool;
}

type SavedItem = Vec<(String, Vec<u8>)>;

struct ProviderIvars {
    url: Retained<NSString>,
    reads: mpsc::UnboundedSender<()>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[name = "PromptheusFileUrlProvider"]
    #[ivars = ProviderIvars]
    struct FileUrlProvider;

    unsafe impl NSObjectProtocol for FileUrlProvider {}

    unsafe impl NSPasteboardItemDataProvider for FileUrlProvider {
        #[unsafe(method(pasteboard:item:provideDataForType:))]
        fn provide_data(
            &self,
            _pasteboard: Option<&NSPasteboard>,
            item: &NSPasteboardItem,
            r#type: &NSPasteboardType,
        ) {
            item.setString_forType(&self.ivars().url, r#type);
            let _ = self.ivars().reads.send(());
        }
    }
);

impl FileUrlProvider {
    fn new(url: Retained<NSString>, reads: mpsc::UnboundedSender<()>) -> Retained<Self> {
        let this = Self::alloc().set_ivars(ProviderIvars { url, reads });
        unsafe { msg_send![super(this), init] }
    }
}

thread_local! {
    static PROVIDERS: RefCell<Vec<Retained<FileUrlProvider>>> = const { RefCell::new(Vec::new()) };
}

pub fn claude_running() -> bool {
    NSWorkspace::sharedWorkspace()
        .runningApplications()
        .iter()
        .any(|app| is_claude(&app))
}

fn is_claude(app: &objc2_app_kit::NSRunningApplication) -> bool {
    app.bundleIdentifier()
        .is_some_and(|id| id.to_string() == CLAUDE_BUNDLE)
}

fn frontmost_is_claude() -> bool {
    NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .is_some_and(|app| is_claude(&app))
}

async fn on_main<T: Send + 'static>(
    app: &AppHandle,
    work: impl FnOnce() -> T + Send + 'static,
) -> Result<T, String> {
    let (tx, rx) = oneshot::channel();
    app.run_on_main_thread(move || {
        let _ = tx.send(work());
    })
    .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())
}

fn save_pasteboard() -> Vec<SavedItem> {
    let Some(items) = NSPasteboard::generalPasteboard().pasteboardItems() else {
        return Vec::new();
    };
    items
        .iter()
        .map(|item| {
            item.types()
                .iter()
                .filter_map(|kind| {
                    let data = item.dataForType(&kind)?;
                    Some((kind.to_string(), data.to_vec()))
                })
                .collect()
        })
        .collect()
}

fn offer_files(paths: Vec<String>, reads: mpsc::UnboundedSender<()>) -> bool {
    let pasteboard = NSPasteboard::generalPasteboard();
    pasteboard.prepareForNewContentsWithOptions(NSPasteboardContentsOptions::CurrentHostOnly);
    let file_url = unsafe { NSPasteboardTypeFileURL };
    let types = NSArray::from_slice(&[file_url]);
    let mut providers = Vec::new();
    let mut items = Vec::new();
    for path in paths {
        let url = NSURL::fileURLWithPath(&NSString::from_str(&path));
        let Some(url) = url.absoluteString() else { continue };
        let provider = FileUrlProvider::new(url, reads.clone());
        let item = NSPasteboardItem::new();
        item.setDataProvider_forTypes(ProtocolObject::from_ref(&*provider), &types);
        items.push(ProtocolObject::<dyn NSPasteboardWriting>::from_retained(item));
        providers.push(provider);
    }
    PROVIDERS.with_borrow_mut(|held| *held = providers);
    pasteboard.writeObjects(&NSArray::from_retained_slice(&items))
}

fn restore_pasteboard(saved: Vec<SavedItem>) {
    let pasteboard = NSPasteboard::generalPasteboard();
    pasteboard.clearContents();
    PROVIDERS.with_borrow_mut(Vec::clear);
    let items: Vec<_> = saved
        .into_iter()
        .filter(|types| !types.is_empty())
        .map(|types| {
            let item = NSPasteboardItem::new();
            for (kind, data) in types {
                item.setData_forType(&NSData::with_bytes(&data), &NSString::from_str(&kind));
            }
            ProtocolObject::<dyn NSPasteboardWriting>::from_retained(item)
        })
        .collect();
    if !items.is_empty() {
        pasteboard.writeObjects(&NSArray::from_retained_slice(&items));
    }
}

fn press(key: CGKeyCode, command: bool) -> Result<(), Outcome> {
    if !frontmost_is_claude() {
        return Err(Outcome::FocusLost);
    }
    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState);
    for down in [true, false] {
        let event = CGEvent::new_keyboard_event(source.as_deref(), key, down)
            .ok_or_else(|| Outcome::Failed("creating the key event failed".into()))?;
        if command {
            CGEvent::set_flags(Some(&event), CGEventFlags::MaskCommand);
        }
        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&event));
    }
    Ok(())
}

async fn wait_for_claude() -> Option<Duration> {
    let start = Instant::now();
    while start.elapsed() < FRONTMOST_TIMEOUT {
        if frontmost_is_claude() {
            return Some(start.elapsed());
        }
        tokio::time::sleep(FRONTMOST_POLL).await;
    }
    None
}

async fn read_arrived(reads: &mut mpsc::UnboundedReceiver<()>) -> bool {
    tokio::time::timeout(READ_TIMEOUT, reads.recv()).await.is_ok()
}

async fn paste_files(reads: &mut mpsc::UnboundedReceiver<()>) -> Result<(), Outcome> {
    while reads.try_recv().is_ok() {}
    press(KEY_V, true)?;
    if read_arrived(reads).await {
        return Ok(());
    }
    log::debug!(target: LOG_TARGET, "no pasteboard read after the first Cmd+V, retrying once");
    press(KEY_V, true)?;
    if read_arrived(reads).await {
        return Ok(());
    }
    Err(Outcome::NoRead)
}

async fn deliver_files(app: &AppHandle, files: &[PathBuf]) -> Result<Vec<SavedItem>, Outcome> {
    let paths: Vec<String> = files.iter().map(|f| f.to_string_lossy().into_owned()).collect();
    let (tx, mut reads) = mpsc::unbounded_channel();
    let main_result = on_main(app, move || {
        let saved = save_pasteboard();
        (saved, offer_files(paths, tx))
    })
    .await
    .map_err(Outcome::Failed)?;
    let (saved, written) = main_result;
    if !written {
        return Err(Outcome::Failed("writing the file list to the pasteboard failed".into()));
    }
    log::debug!(target: LOG_TARGET, "file list on the pasteboard, items saved={}", saved.len());
    paste_files(&mut reads).await?;
    log::debug!(target: LOG_TARGET, "pasteboard read counted after Cmd+V");
    Ok(saved)
}

pub async fn paste_into_claude(app: &AppHandle, files: &[PathBuf], warm: bool) -> Outcome {
    if !unsafe { AXIsProcessTrusted() } {
        return Outcome::NoPermission;
    }
    let Some(found_after) = wait_for_claude().await else {
        return Outcome::NoWindow;
    };
    log::debug!(target: LOG_TARGET, "Claude Desktop frontmost after {} ms", found_after.as_millis());
    tokio::time::sleep(if warm { SETTLE_WARM } else { SETTLE_COLD }).await;

    let saved = if files.is_empty() {
        None
    } else {
        match deliver_files(app, files).await {
            Ok(saved) => Some(saved),
            Err(outcome) => return outcome,
        }
    };

    if let Err(outcome) = press(KEY_RETURN, false) {
        return outcome;
    }
    log::debug!(target: LOG_TARGET, "Return sent");

    if let Some(saved) = saved {
        tokio::time::sleep(RESTORE_DELAY).await;
        if let Err(e) = on_main(app, move || restore_pasteboard(saved)).await {
            return Outcome::Failed(e);
        }
        log::debug!(target: LOG_TARGET, "pasteboard restored");
    }
    Outcome::Sent
}
