use std::sync::OnceLock;

use zbus::Connection;

#[zbus::proxy(
    interface = "com.promptheus.Shell",
    default_service = "org.gnome.Shell",
    default_path = "/com/promptheus/Shell"
)]
pub trait Shell {
    fn set_shortcuts(
        &self,
        shortcuts: std::collections::HashMap<String, Vec<String>>,
    ) -> zbus::Result<Vec<String>>;

    fn get_pointer(&self) -> zbus::Result<(i32, i32, i32, i32, i32, i32)>;

    fn place_window(&self, title: &str, x: i32, y: i32, activate: bool) -> zbus::Result<bool>;

    fn get_focused_wm_class(&self) -> zbus::Result<String>;

    #[zbus(signal)]
    fn shortcut_activated(&self, action: String) -> zbus::Result<()>;

    #[zbus(signal)]
    fn ready(&self) -> zbus::Result<()>;
}

static CONNECTION: OnceLock<Connection> = OnceLock::new();

fn is_gnome_wayland_session(session_type: Option<&str>, current_desktop: Option<&str>) -> bool {
    session_type == Some("wayland")
        && current_desktop.is_some_and(|d| d.split(':').any(|part| part == "GNOME"))
}

pub fn is_gnome_wayland() -> bool {
    is_gnome_wayland_session(
        std::env::var("XDG_SESSION_TYPE").ok().as_deref(),
        std::env::var("XDG_CURRENT_DESKTOP").ok().as_deref(),
    )
}

async fn connection() -> zbus::Result<Connection> {
    if let Some(connection) = CONNECTION.get() {
        return Ok(connection.clone());
    }
    let connection = Connection::session().await?;
    log::debug!("connected to the session bus");
    Ok(CONNECTION.get_or_init(|| connection).clone())
}

pub async fn proxy() -> zbus::Result<ShellProxy<'static>> {
    ShellProxy::new(&connection().await?).await
}

pub fn blocking_proxy() -> zbus::Result<ShellProxyBlocking<'static>> {
    let connection = zbus::block_on(connection())?;
    ShellProxyBlocking::new(&zbus::blocking::Connection::from(connection))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wayland_with_gnome_is_gnome_wayland() {
        assert!(is_gnome_wayland_session(Some("wayland"), Some("ubuntu:GNOME")));
    }

    #[test]
    fn x11_with_gnome_is_not_gnome_wayland() {
        assert!(!is_gnome_wayland_session(Some("x11"), Some("GNOME")));
    }

    #[test]
    fn wayland_with_kde_is_not_gnome_wayland() {
        assert!(!is_gnome_wayland_session(Some("wayland"), Some("KDE")));
    }

    #[test]
    fn missing_variables_are_not_gnome_wayland() {
        assert!(!is_gnome_wayland_session(None, None));
        assert!(!is_gnome_wayland_session(Some("wayland"), None));
        assert!(!is_gnome_wayland_session(None, Some("GNOME")));
    }
}
