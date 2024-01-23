use gamescope_x11_client::xwayland::{BlurMode, Primary, XWayland};
use std::error::Error;
use zbus::fdo;
use zbus_macros::dbus_interface;

/// DBus interface imeplementation for Gamescope XWayland instance
pub struct DBusInterface {
    xwayland: XWayland,
}

impl DBusInterface {
    /// Returns a new instance of the XWayland DBus interface. Will error if
    /// it cannot establish a connection.
    pub fn new(name: String) -> Result<DBusInterface, Box<dyn Error>> {
        let mut xwayland = XWayland::new(name);
        xwayland.connect()?;
        Ok(DBusInterface { xwayland })
    }
}

#[dbus_interface(name = "org.shadowblip.Gamescope.XWayland")]
impl DBusInterface {
    #[dbus_interface(property)]
    async fn name(&self) -> fdo::Result<String> {
        Ok(self.xwayland.get_name())
    }

    #[dbus_interface(property)]
    pub async fn primary(&self) -> fdo::Result<bool> {
        let value = self
            .xwayland
            .is_primary_instance()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value)
    }

    #[dbus_interface(property)]
    async fn root_window_id(&self) -> fdo::Result<u32> {
        let value = self
            .xwayland
            .get_root_window_id()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value)
    }

    async fn get_window_name(&self, window_id: u32) -> fdo::Result<String> {
        let name = self
            .xwayland
            .get_window_name(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(name.unwrap_or_default())
    }

    async fn get_window_children(&self, window_id: u32) -> fdo::Result<Vec<u32>> {
        let value = self
            .xwayland
            .get_window_children(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value)
    }

    async fn get_all_windows(&self, window_id: u32) -> fdo::Result<Vec<u32>> {
        let value = self
            .xwayland
            .get_all_windows(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value)
    }

    async fn get_app_id(&self, window_id: u32) -> fdo::Result<u32> {
        let value = self
            .xwayland
            .get_app_id(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value.unwrap_or_default())
    }

    async fn set_app_id(&self, window_id: u32, app_id: u32) -> fdo::Result<()> {
        self.xwayland
            .set_app_id(window_id, app_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    async fn has_app_id(&self, window_id: u32) -> fdo::Result<bool> {
        let value = self
            .xwayland
            .has_app_id(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value)
    }
}

/// DBus interface imeplementation for primary Gamescope XWayland instance
pub struct DBusInterfacePrimary {
    xwayland: XWayland,
}

impl DBusInterfacePrimary {
    /// Returns a new instance of the XWayland DBus interface. Will error if
    /// it cannot establish a connection.
    pub fn new(name: String) -> Result<DBusInterfacePrimary, Box<dyn Error>> {
        let mut xwayland = XWayland::new(name);
        xwayland.connect()?;
        Ok(DBusInterfacePrimary { xwayland })
    }
}

#[dbus_interface(name = "org.shadowblip.Gamescope.XWayland.Primary")]
impl DBusInterfacePrimary {
    #[dbus_interface(property)]
    async fn focusable_apps(&self) -> fdo::Result<Vec<u32>> {
        let value = self
            .xwayland
            .get_focusable_apps()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value.unwrap_or_default())
    }

    #[dbus_interface(property)]
    async fn focusable_windows(&self) -> fdo::Result<Vec<u32>> {
        let value = self
            .xwayland
            .get_focusable_windows()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value.unwrap_or_default())
    }

    #[dbus_interface(property)]
    async fn focusable_window_names(&self) -> fdo::Result<Vec<String>> {
        let value = self
            .xwayland
            .get_focusable_window_names()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value)
    }

    #[dbus_interface(property)]
    async fn focused_window(&self) -> fdo::Result<u32> {
        let value = self
            .xwayland
            .get_focused_window()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value.unwrap_or_default())
    }

    #[dbus_interface(property)]
    async fn focused_app(&self) -> fdo::Result<u32> {
        let value = self
            .xwayland
            .get_focused_app()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value.unwrap_or_default())
    }

    #[dbus_interface(property)]
    async fn focused_app_gfx(&self) -> fdo::Result<u32> {
        let value = self
            .xwayland
            .get_focused_app_gfx()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value.unwrap_or_default())
    }

    #[dbus_interface(property)]
    async fn overlay_focused(&self) -> fdo::Result<bool> {
        let value = self
            .xwayland
            .is_overlay_focused()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value)
    }

    #[dbus_interface(property)]
    async fn fps_limit(&self) -> fdo::Result<u32> {
        let value = self
            .xwayland
            .get_fps_limit()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value.unwrap_or_default())
    }

    #[dbus_interface(property)]
    async fn set_fps_limit(&mut self, fps: u32) -> fdo::Result<()> {
        self.xwayland
            .set_fps_limit(fps)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    #[dbus_interface(property)]
    async fn blur_mode(&self) -> fdo::Result<u32> {
        let value = self
            .xwayland
            .get_blur_mode()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        if value.is_none() {
            return Ok(0);
        }
        match value.unwrap() {
            BlurMode::Off => Ok(0),
            BlurMode::Cond => Ok(1),
            BlurMode::Always => Ok(2),
        }
    }

    #[dbus_interface(property)]
    async fn set_blur_mode(&mut self, mode: u32) -> fdo::Result<()> {
        let blur_mode = match mode {
            0 => BlurMode::Off,
            1 => BlurMode::Cond,
            2 => BlurMode::Always,
            _ => BlurMode::Off,
        };
        self.xwayland
            .set_blur_mode(blur_mode)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    #[dbus_interface(property)]
    async fn blur_radius(&self) -> fdo::Result<u32> {
        //let value = self
        //    .xwayland
        //    .get_blur_radius()
        //    .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(0)
    }

    #[dbus_interface(property)]
    async fn set_blur_radius(&mut self, radius: u32) -> fdo::Result<()> {
        self.xwayland
            .set_blur_radius(radius)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    #[dbus_interface(property)]
    async fn allow_tearing(&self) -> fdo::Result<bool> {
        //let value = self
        //    .xwayland
        //    .get_allow_tearing()
        //    .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(false)
    }

    #[dbus_interface(property)]
    async fn set_allow_tearing(&mut self, allow: bool) -> fdo::Result<()> {
        self.xwayland
            .set_allow_tearing(allow)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    async fn is_focusable_app(&self, window_id: u32) -> fdo::Result<bool> {
        let value = self
            .xwayland
            .is_focusable_app(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value)
    }

    async fn set_main_app(&self, window_id: u32) -> fdo::Result<()> {
        self.xwayland
            .set_main_app(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    async fn set_input_focus(&self, window_id: u32, value: u32) -> fdo::Result<()> {
        self.xwayland
            .set_input_focus(window_id, value)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    async fn get_overlay(&self, window_id: u32) -> fdo::Result<u32> {
        let value = self
            .xwayland
            .get_overlay(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value.unwrap_or_default())
    }

    async fn set_overlay(&self, window_id: u32, value: u32) -> fdo::Result<()> {
        self.xwayland
            .set_overlay(window_id, value)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    async fn set_notification(&self, window_id: u32, value: u32) -> fdo::Result<()> {
        self.xwayland
            .set_notification(window_id, value)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    async fn set_external_overlay(&self, window_id: u32, value: u32) -> fdo::Result<()> {
        self.xwayland
            .set_external_overlay(window_id, value)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    async fn get_baselayer_window(&self) -> fdo::Result<u32> {
        let value = self
            .xwayland
            .get_baselayer_window()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value.unwrap_or_default())
    }

    async fn set_baselayer_window(&self, window_id: u32) -> fdo::Result<()> {
        self.xwayland
            .set_baselayer_window(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    async fn remove_baselayer_window(&self) -> fdo::Result<()> {
        self.xwayland
            .remove_baselayer_window()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    async fn request_screenshot(&self) -> fdo::Result<()> {
        self.xwayland
            .request_screenshot()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }
}
