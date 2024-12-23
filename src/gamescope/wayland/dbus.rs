use std::error::Error;

use gamescope_wayland_client::control::gamescope_control::ScreenshotType;
use zbus::{dbus_interface, fdo, Connection};

use super::manager::{screenshot_type_from_u8, WaylandManager, WaylandMessage};

/// DBus interface implementation for Gamescope Wayland instance.
#[allow(dead_code)]
pub struct DBusInterface {
    path: String,
    wayland: WaylandManager,
    dbus: Connection,
}

#[allow(dead_code)]
impl DBusInterface {
    /// Returns a new instance of the Wayland DBus interface. Will error if
    /// it cannot establish a connection.
    pub async fn new(
        path: String,
        dbus: Connection,
        socket_path: String,
    ) -> Result<DBusInterface, Box<dyn Error>> {
        let wayland = WaylandManager::new(socket_path).await?;

        Ok(DBusInterface {
            path,
            wayland,
            dbus,
        })
    }

    /// Returns a reference to the dbus interface
    async fn get_interface(&self) -> Result<zbus::InterfaceRef<DBusInterface>, zbus::Error> {
        self.dbus
            .clone()
            .object_server()
            .interface::<_, DBusInterface>(self.path.clone())
            .await
    }
}

#[dbus_interface(name = "org.shadowblip.Gamescope.Wayland")]
impl DBusInterface {
    /// Takes a screenshot using Wayland
    /// the screenshot_type u8 converts to [ScreenshotType]
    /// 0 => [ScreenshotType::AllRealLayers]
    /// 1 => [ScreenshotType::BasePlaneOnly]
    /// 2 => [ScreenshotType::FullComposition]
    /// 3 => [ScreenshotType::ScreenBuffer]
    pub async fn take_screenshot(&self, file_path: String, screenshot_type: u8) -> fdo::Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel::<Result<(), String>>(16);
        let Some(screenshot_type): Option<ScreenshotType> =
            screenshot_type_from_u8(screenshot_type)
        else {
            return Err(fdo_error("Invalid screenshot type"));
        };

        self.wayland
            .send(WaylandMessage::CommandTakeScreenshot(
                tx,
                file_path,
                screenshot_type,
            ))
            .await
            .map_err(|err| to_fdo_error("Error when sending screenshot command", err))?;

        match rx.recv().await {
            Some(Ok(_)) => {
                log::info!("Screenshot taken");
                Ok(())
            }
            Some(Err(err)) => Err(to_fdo_error("Error from screenshot command", err.into())),
            None => Err(fdo_error("No response received for screenshot command")),
        }
    }
}

fn to_fdo_error(description: &str, err: Box<dyn Error>) -> fdo::Error {
    let err = format!("{description}, err:{err:?}");
    log::error!("{err}");
    fdo::Error::Failed(err)
}

fn fdo_error(description: &str) -> fdo::Error {
    log::error!("{description}");
    fdo::Error::Failed(description.to_owned())
}
