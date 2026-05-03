#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub use macos::detect;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub use linux::detect;

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod fallback;
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub use fallback::detect;
