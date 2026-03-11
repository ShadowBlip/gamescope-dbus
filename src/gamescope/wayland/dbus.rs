use std::error::Error;

use gamescope_wayland_client::control::gamescope_control::{DisplaySleepFlags, ScreenshotType};
use zbus::{dbus_interface, fdo, Connection};

use crate::gamescope::wayland::manager::{display_type_from_u8, target_refresh_cycle_from_u8};

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
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();
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

        match rx.await {
            Ok(Ok(_)) => {
                log::info!("Screenshot taken");
                Ok(())
            }
            Ok(Err(err)) => Err(to_fdo_error("Error from screenshot command", err.into())),
            Err(_) => Err(fdo_error("No response received for screenshot command")),
        }
    }

    /// Sleeps chosen display
    /// display_type_flags u8 converts to DisplayTypeFlags
    /// It is a bit flag
    /// 1 => [DisplayTypeFlags::InternalDisplay]
    /// 2 => [DisplayTypeFlags::ExternalDisplay]
    /// [sleep] - whether to sleep - true or wake - false
    pub async fn display_sleep(&self, display_type_flags: u8, sleep: bool) -> fdo::Result<()> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

        let Some(display_flags) = display_type_from_u8(display_type_flags) else {
            return Err(fdo_error("Invalid display type"));
        };

        let sleep_flag = if sleep {
            DisplaySleepFlags::Sleep
        } else {
            DisplaySleepFlags::Wake
        };

        self.wayland
            .send(WaylandMessage::DisplaySleep(tx, display_flags, sleep_flag))
            .await
            .map_err(|err| to_fdo_error("Error when sending sleep command", err))?;

        match rx.await {
            Ok(Ok(_)) => {
                log::info!("Screen sleeping");
                Ok(())
            }
            Ok(Err(err)) => Err(to_fdo_error("Error from screen sleep", err.into())),
            Err(_) => Err(fdo_error("No response from sleep command")),
        }
    }

    /// Updates in-game FPS limit and refresh rate
    /// fps is a number of fps to set
    /// set to 0 to use a native default
    /// refresh_cycle_flags u8 converts to TargetRefreshCycleFlag
    /// It is a bit flag
    /// 1 => [TargetRefreshCycleFlag::InternalDisplay]
    /// 2 => [TargetRefreshCycleFlag::AllowRefreshSwitching]
    /// 4 => [TargetRefreshCycleFlag::OnlyChangeRefreshRate]
    pub async fn set_app_target_refresh_cycle(
        &self,
        fps: u32,
        refresh_cycle_flags: u8,
    ) -> fdo::Result<()> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<(), String>>();

        let flags = target_refresh_cycle_from_u8(refresh_cycle_flags);

        self.wayland
            .send(WaylandMessage::SetAppTargetRefreshCycle(tx, fps, flags))
            .await
            .map_err(|err| to_fdo_error("Error when sending fps limit command", err))?;

        match rx.await {
            Ok(Ok(_)) => {
                log::info!("Fps limit submited");
                Ok(())
            }
            Ok(Err(err)) => Err(to_fdo_error("Error from fps limit", err.into())),
            Err(_) => Err(fdo_error("No response from fps limit command")),
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
