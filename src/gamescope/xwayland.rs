use gamescope_x11_client::{
    atoms::GamescopeAtom,
    xwayland::{BlurMode, Primary, XWayland},
};
use std::{collections::HashMap, error::Error, sync::mpsc::Receiver};
use tokio::task::AbortHandle;
use zbus::{fdo, Connection, SignalContext};
use zbus_macros::dbus_interface;

/// DBus interface imeplementation for Gamescope XWayland instance
pub struct DBusInterface {
    path: String,
    xwayland: XWayland,
    dbus: Connection,
    watched_windows: Vec<u32>,
    watch_handles: HashMap<u32, AbortHandle>,
}

impl DBusInterface {
    /// Returns a new instance of the XWayland DBus interface. Will error if
    /// it cannot establish a connection.
    pub fn new(
        name: String,
        path: String,
        dbus: Connection,
    ) -> Result<DBusInterface, Box<dyn Error>> {
        let mut xwayland = XWayland::new(name);
        xwayland.connect()?;
        let watched_windows = Vec::new();
        Ok(DBusInterface {
            path,
            xwayland,
            watched_windows,
            dbus,
            watch_handles: HashMap::new(),
        })
    }
}

#[dbus_interface(name = "org.shadowblip.Gamescope.XWayland")]
impl DBusInterface {
    /// The X display name of the XWayland display (E.g. ":0", ":1")
    #[dbus_interface(property)]
    async fn name(&self) -> fdo::Result<String> {
        Ok(self.xwayland.get_name())
    }

    /// Returns true if this instance is the primary Gamescope xwayland instance
    #[dbus_interface(property)]
    pub async fn primary(&self) -> fdo::Result<bool> {
        let value = self
            .xwayland
            .is_primary_instance()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value)
    }

    /// Returns the root window ID of the xwayland instance
    #[dbus_interface(property)]
    async fn root_window_id(&self) -> fdo::Result<u32> {
        let value = self
            .xwayland
            .get_root_window_id()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value)
    }

    /// List of windows currently being watched for property changes. The
    /// [WindowPropertyChanged] signal will fire whenever one of these windows
    /// has a property change.
    #[dbus_interface(property)]
    async fn watched_windows(&self) -> Vec<u32> {
        self.watched_windows.clone()
    }

    /// Emitted when a window property changes on a watched window.
    #[dbus_interface(signal)]
    async fn window_property_changed(
        ctxt: &SignalContext<'_>,
        window: u32,
        prop: String,
    ) -> zbus::Result<()>;

    /// Start watching the given window. The [WindowPropertyChanged] signal
    /// will fire whenever a window property changes on the window. Use
    /// [UnwatchWindow] to stop watching the given window.
    async fn watch_window(&mut self, window_id: u32) -> fdo::Result<()> {
        // If the window is already being watched, do nothing
        if self.watched_windows.contains(&window_id) {
            return Ok(());
        }

        // Spawn a new thread to listen for property changes for the given window
        let (_, rx) = self
            .xwayland
            .listen_for_window_property_changes(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;

        // Create a closure to run whenever a property changes on this window
        let dispatch_to_dbus = |conn: Connection, path: String, event: String, id: u32| {
            tokio::task::spawn(async move {
                // Get the object instance at the given path so we can send DBus signal
                // updates
                let iface_ref = conn
                    .object_server()
                    .interface::<_, DBusInterface>(path)
                    .await
                    .expect("Unable to get reference to DBus interface");

                log::debug!("Got property change event: {:?}", event);

                // Emit the property changed signal for this window
                DBusInterface::window_property_changed(iface_ref.signal_context(), id, event)
                    .await
                    .unwrap_or_else(|error| {
                        log::warn!("Unable to signal value change: {:?}", error)
                    });
            });
        };

        // Spawn a task to process the messages in the receiver
        let conn = self.dbus.clone();
        let path = self.path.clone();
        let handle = tokio::task::spawn_blocking(move || {
            log::debug!(
                "Started listening for property changes for window {}",
                window_id
            );

            // Wait for events from the channel and dispatch them to the DBus interface
            while let Ok(event) = rx.recv() {
                log::debug!("Got property change event: {:?}", event);
                dispatch_to_dbus(conn.clone(), path.clone(), event, window_id);
            }
            log::warn!("Stopped listening for property changes");
        })
        .abort_handle();

        // Add to the list of windows we're watching
        self.watched_windows.push(window_id);
        self.watch_handles.insert(window_id, handle);

        Ok(())
    }

    /// Stop watching the given window. The [WindowPropertyChanged] signal will
    /// no longer fire for the given window.
    async fn unwatch_window(&mut self, window_id: u32) -> fdo::Result<()> {
        let index = self.watched_windows.iter().position(|x| *x == window_id);
        if index.is_none() {
            return Ok(());
        }

        // Remove the element and stop watching
        self.watched_windows.remove(index.unwrap());
        let handle = self.watch_handles.remove(&window_id).unwrap();
        handle.abort();

        Ok(())
    }

    /// Discover the process IDs that are associated with the given window
    async fn get_pids_for_window(&self, window_id: u32) -> fdo::Result<Vec<u32>> {
        let value = self
            .xwayland
            .get_pids_for_window(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value)
    }

    /// Returns the window id(s) for the given process ID.
    async fn get_windows_for_pid(&self, pid: u32) -> fdo::Result<Vec<u32>> {
        let value = self
            .xwayland
            .get_windows_for_pid(pid)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value)
    }

    /// Returns the window name of the given window
    async fn get_window_name(&self, window_id: u32) -> fdo::Result<String> {
        let name = self
            .xwayland
            .get_window_name(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(name.unwrap_or_default())
    }

    /// Returns the window ids of the children of the given window
    async fn get_window_children(&self, window_id: u32) -> fdo::Result<Vec<u32>> {
        let value = self
            .xwayland
            .get_window_children(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value)
    }

    /// Recursively returns all child windows of the given window id
    async fn get_all_windows(&self, window_id: u32) -> fdo::Result<Vec<u32>> {
        let value = self
            .xwayland
            .get_all_windows(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value)
    }

    /// Returns the currently set app ID on the given window
    async fn get_app_id(&self, window_id: u32) -> fdo::Result<u32> {
        let value = self
            .xwayland
            .get_app_id(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value.unwrap_or_default())
    }

    /// Sets the app ID on the given window
    async fn set_app_id(&self, window_id: u32, app_id: u32) -> fdo::Result<()> {
        self.xwayland
            .set_app_id(window_id, app_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    /// Returns whether or not the given window has an app ID set
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

    /// Starts a new thread listening for gamescope property changes. Returns
    /// a receiver channel where changes will be sent to. This is usually used
    /// to process DBus property changes outside of the dispatched handler
    pub fn listen_for_property_changes(&self) -> Result<Receiver<String>, Box<dyn Error>> {
        let (_, rx) = self.xwayland.listen_for_property_changes()?;
        Ok(rx)
    }
}

#[dbus_interface(name = "org.shadowblip.Gamescope.XWayland.Primary")]
impl DBusInterfacePrimary {
    /// Return a list of focusable apps
    #[dbus_interface(property)]
    async fn focusable_apps(&self) -> fdo::Result<Vec<u32>> {
        let value = self
            .xwayland
            .get_focusable_apps()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value.unwrap_or_default())
    }

    /// Returns a list of focusable window ids
    #[dbus_interface(property)]
    async fn focusable_windows(&self) -> fdo::Result<Vec<u32>> {
        let value = self
            .xwayland
            .get_focusable_windows()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value.unwrap_or_default())
    }

    /// Returns a list of focusable window names
    #[dbus_interface(property)]
    async fn focusable_window_names(&self) -> fdo::Result<Vec<String>> {
        let value = self
            .xwayland
            .get_focusable_window_names()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value)
    }

    /// Return the currently focused window id.
    #[dbus_interface(property)]
    async fn focused_window(&self) -> fdo::Result<u32> {
        let value = self
            .xwayland
            .get_focused_window()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value.unwrap_or_default())
    }

    /// Return the currently focused app id.
    #[dbus_interface(property)]
    async fn focused_app(&self) -> fdo::Result<u32> {
        let value = self
            .xwayland
            .get_focused_app()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value.unwrap_or_default())
    }

    /// Return the currently focused gfx app id.
    #[dbus_interface(property)]
    async fn focused_app_gfx(&self) -> fdo::Result<u32> {
        let value = self
            .xwayland
            .get_focused_app_gfx()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value.unwrap_or_default())
    }

    /// Returns whether or not the overlay window is currently focused
    #[dbus_interface(property)]
    async fn overlay_focused(&self) -> fdo::Result<bool> {
        let value = self
            .xwayland
            .is_overlay_focused()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value)
    }

    /// The current Gamescope FPS limit
    #[dbus_interface(property)]
    async fn fps_limit(&self) -> fdo::Result<u32> {
        let value = self
            .xwayland
            .get_fps_limit()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value.unwrap_or_default())
    }

    /// Sets the current Gamescope FPS limit
    #[dbus_interface(property)]
    async fn set_fps_limit(&mut self, fps: u32) -> fdo::Result<()> {
        self.xwayland
            .set_fps_limit(fps)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    /// The Gamescope blur mode (0 - off, 1 - cond, 2 - always)
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

    /// Sets the Gamescope blur mode
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

    /// The blur radius size
    #[dbus_interface(property)]
    async fn blur_radius(&self) -> fdo::Result<u32> {
        //let value = self
        //    .xwayland
        //    .get_blur_radius()
        //    .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(0)
    }

    /// Sets the blur radius size
    #[dbus_interface(property)]
    async fn set_blur_radius(&mut self, radius: u32) -> fdo::Result<()> {
        self.xwayland
            .set_blur_radius(radius)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    /// Whether or not Gamescope should be allowed to screen tear
    #[dbus_interface(property)]
    async fn allow_tearing(&self) -> fdo::Result<bool> {
        //let value = self
        //    .xwayland
        //    .get_allow_tearing()
        //    .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(false)
    }

    /// Sets whether or not Gamescope should be allowed to screen tear
    #[dbus_interface(property)]
    async fn set_allow_tearing(&mut self, allow: bool) -> fdo::Result<()> {
        self.xwayland
            .set_allow_tearing(allow)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    /// Returns true if the window with the given window ID exists in focusable apps
    #[dbus_interface(out_args("is_focusable"))]
    async fn is_focusable_app(&self, window_id: u32) -> fdo::Result<bool> {
        let value = self
            .xwayland
            .is_focusable_app(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value)
    }

    /// Fires when the baselayer window has been updated
    #[dbus_interface(signal)]
    async fn baselayer_window_updated(ctxt: &SignalContext<'_>) -> zbus::Result<()>;

    /// Sets the given window as the main launcher app. This will set an X window
    /// property called STEAM_GAME to 769 (Steam), which will make Gamescope
    /// treat the window as the main overlay.
    async fn set_main_app(&self, window_id: u32) -> fdo::Result<()> {
        self.xwayland
            .set_main_app(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    /// Set the given window as the primary overlay input focus. This should be set to
    /// "1" whenever the overlay wants to intercept input from a game.
    async fn set_input_focus(&self, window_id: u32, value: u32) -> fdo::Result<()> {
        self.xwayland
            .set_input_focus(window_id, value)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    /// Get the overlay status for the given window
    async fn get_overlay(&self, window_id: u32) -> fdo::Result<u32> {
        let value = self
            .xwayland
            .get_overlay(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value.unwrap_or_default())
    }

    /// Set the given window as the main overlay window
    async fn set_overlay(&self, window_id: u32, value: u32) -> fdo::Result<()> {
        self.xwayland
            .set_overlay(window_id, value)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    /// Set the given window as a notification. This should be set to "1" when some
    /// UI wants to be shown but not intercept input.
    async fn set_notification(&self, window_id: u32, value: u32) -> fdo::Result<()> {
        self.xwayland
            .set_notification(window_id, value)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    /// Set the given window as an external overlay window
    async fn set_external_overlay(&self, window_id: u32, value: u32) -> fdo::Result<()> {
        self.xwayland
            .set_external_overlay(window_id, value)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    /// Returns the currently set manual focus
    async fn get_baselayer_window(&self) -> fdo::Result<u32> {
        let value = self
            .xwayland
            .get_baselayer_window()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(value.unwrap_or_default())
    }

    /// Focuses the given window
    async fn set_baselayer_window(&self, window_id: u32) -> fdo::Result<()> {
        self.xwayland
            .set_baselayer_window(window_id)
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    /// Removes the baselayer property to un-focus windows
    async fn remove_baselayer_window(&self) -> fdo::Result<()> {
        self.xwayland
            .remove_baselayer_window()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }

    /// Request a screenshot from Gamescope
    async fn request_screenshot(&self) -> fdo::Result<()> {
        self.xwayland
            .request_screenshot()
            .map_err(|err| fdo::Error::Failed(err.to_string()))?;
        Ok(())
    }
}

/// Listen for property changes and emit the appropriate DBus signals. This is
/// split into two methods to bridge the gap between the sync world and the async
/// world.
pub async fn dispatch_primary_property_changes(
    conn: zbus::Connection,
    path: String,
    rx: Receiver<String>,
) -> Result<(), Box<dyn Error>> {
    tokio::task::spawn_blocking(move || {
        log::debug!("Started listening for property changes");

        // Wait for events from the channel and dispatch them to the DBus interface
        while let Ok(event) = rx.recv() {
            log::debug!("Got property change event: {:?}", event);
            dispatch_primary_to_dbus(conn.clone(), path.clone(), event);
        }
        log::warn!("Stopped listening for property changes");
    });

    Ok(())
}

/// Dispatch the given event to DBus using async
fn dispatch_primary_to_dbus(conn: zbus::Connection, path: String, event: String) {
    tokio::task::spawn(async move {
        // Get the object instance at the given path so we can send DBus signal
        // updates
        let iface_ref = conn
            .object_server()
            .interface::<_, DBusInterfacePrimary>(path)
            .await
            .expect("Unable to get reference to DBus interface");

        log::debug!("Got property change event: {:?}", event);
        let iface = iface_ref.get_mut().await;

        // Match on the type of property that was changed to send the appropriate
        // DBus signal.
        // NOTE: These should only be defined for "read-only" properties
        // TODO: Maybe this can be automatically expressed better using a macro
        if event == GamescopeAtom::FocusedApp.to_string() {
            iface
                .focused_app_changed(iface_ref.signal_context())
                .await
                .unwrap_or_else(|error| log::warn!("Unable to signal value change: {:?}", error));
        } else if event == GamescopeAtom::FocusableApps.to_string() {
            iface
                .focusable_apps_changed(iface_ref.signal_context())
                .await
                .unwrap_or_else(|error| log::warn!("Unable to signal value change: {:?}", error));
        } else if event == GamescopeAtom::FocusedAppGFX.to_string() {
            iface
                .focused_app_gfx_changed(iface_ref.signal_context())
                .await
                .unwrap_or_else(|error| log::warn!("Unable to signal value change: {:?}", error));
        } else if event == GamescopeAtom::FocusedWindow.to_string() {
            iface
                .focused_window_changed(iface_ref.signal_context())
                .await
                .unwrap_or_else(|error| log::warn!("Unable to signal value change: {:?}", error));
        } else if event == GamescopeAtom::FocusableWindows.to_string() {
            iface
                .focusable_windows_changed(iface_ref.signal_context())
                .await
                .unwrap_or_else(|error| log::warn!("Unable to signal value change: {:?}", error));
        } else if event == GamescopeAtom::BaselayerWindow.to_string() {
            DBusInterfacePrimary::baselayer_window_updated(iface_ref.signal_context())
                .await
                .unwrap_or_else(|error| log::warn!("Unable to signal value change: {:?}", error));
        }
    });
}
