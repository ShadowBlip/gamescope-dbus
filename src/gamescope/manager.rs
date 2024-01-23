use std::error::Error;
use zbus::{fdo, Connection};
use zbus_macros::dbus_interface;

use super::xwayland;

/// DBus interface imeplementation for Gamescope Manager instance
pub struct DBusInterface {
    dbus: Connection,
}

impl DBusInterface {
    /// Returns a new instance of the XWayland DBus interface. Will error if
    /// it cannot establish a connection.
    pub fn new(conn: Connection) -> DBusInterface {
        // TODO: Use inotify to watch for new gamescope instances

        DBusInterface { dbus: conn }
    }

    /// Discovers and adds xwayland interfaces
    pub async fn add_xwaylands(&self) -> Result<(), Box<dyn Error>> {
        // Discover new gamescopes
        // TODO: Handle errors
        let xwaylands_names = gamescope_x11_client::discover_gamescope_displays().unwrap();
        for (i, name) in xwaylands_names.into_iter().enumerate() {
            let xwayland = xwayland::DBusInterface::new(name)?;
            let path = format!("/org/shadowblip/Gamescope/XWayland{}", i);
            self.dbus.object_server().at(path, xwayland).await?;
        }

        Ok(())
    }
}

#[dbus_interface(name = "org.shadowblip.Gamescope.Manager")]
impl DBusInterface {
    #[dbus_interface(property)]
    async fn name(&self) -> fdo::Result<String> {
        Ok("Woo".into())
    }
}
