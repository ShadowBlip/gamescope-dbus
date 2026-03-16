use std::error::Error;

use gamescope_wayland_client::control::gamescope_control::{
    DisplaySleepFlags, DisplayTypeFlags, ScreenshotType, TargetRefreshCycleFlag,
};
use zbus::{dbus_interface, fdo, Connection};

use super::manager::{
    screenshot_type_from_u8, WaylandManager, WaylandMessage, WaylandPropertyChanges,
};

/// DBus interface implementation for Gamescope Wayland instance.
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
        let mut wayland = WaylandManager::new(socket_path).await?;

        dispatch_property_change_to_dbus(
            dbus.clone(),
            path.clone(),
            wayland.property_dispatch_rx.take().unwrap(),
        );

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
    #[dbus_interface(property)]
    pub async fn refresh_rates(&self) -> fdo::Result<Vec<u32>> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Option<Vec<u32>>>();
        self.wayland
            .send(WaylandMessage::PropertyRefreshRates(tx))
            .await
            .map_err(|err| to_fdo_error("Error when sending refresh rates get command", err))?;

        match rx.await {
            Ok(rates) => Ok(rates.unwrap_or_default()),
            Err(_) => Err(fdo_error("No response received for refresh rates property")),
        }
    }

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

        let Some(display_flags) = DisplayTypeFlags::from_bits(display_type_flags as u32) else {
            return Err(fdo_error("Invalid display type"));
        };

        let sleep_flag = if sleep {
            DisplaySleepFlags::Sleep
        } else {
            DisplaySleepFlags::Wake
        };

        self.wayland
            .send(WaylandMessage::CommandDisplaySleep(
                tx,
                display_flags,
                sleep_flag,
            ))
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

        let flags = TargetRefreshCycleFlag::from_bits_truncate(refresh_cycle_flags as u32);

        self.wayland
            .send(WaylandMessage::CommandSetAppTargetRefreshCycle(
                tx, fps, flags,
            ))
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

    // Requests frametime in ns for given app
    // Error if invalid app_id
    pub async fn request_app_performance_stats(&self, app_id: u32) -> fdo::Result<u64> {
        let (tx, rx) = tokio::sync::oneshot::channel::<Result<u64, String>>();

        self.wayland
            .send(WaylandMessage::CommandRequestAppPerformanceStats(
                tx, app_id,
            ))
            .await
            .map_err(|err| to_fdo_error("Error when requesting performance stats", err))?;

        match rx.await {
            Ok(Ok(frametime)) => Ok(frametime),
            Ok(Err(err)) => Err(to_fdo_error("Error from performance command", err.into())),
            Err(_) => Err(fdo_error("No response from performance command")),
        }
    }
}

/// Dispatch the given event to DBus using async context
fn dispatch_property_change_to_dbus(
    conn: zbus::Connection,
    path: String,
    mut rx: tokio::sync::mpsc::Receiver<WaylandPropertyChanges>,
) {
    tokio::task::spawn(async move {
        while let Some(property) = rx.recv().await {
            // Get the object instance at the given path so we can send DBus signal
            // updates
            let Ok(iface_ref) = conn
                .object_server()
                .interface::<_, DBusInterface>(path.clone())
                .await
            else {
                log::warn!("Not able to find dbus interface to dispatch property change event");
                return;
            };

            let iface = iface_ref.get_mut().await;

            match property {
                WaylandPropertyChanges::RefreshRates => {
                    iface.refresh_rates_changed(iface_ref.signal_context())
                }
            }
            .await
            .ok();
        }
    });
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
