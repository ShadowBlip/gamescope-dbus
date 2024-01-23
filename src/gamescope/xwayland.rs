use gamescope_x11_client::xwayland::XWayland;
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
}
