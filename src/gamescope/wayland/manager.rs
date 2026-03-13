use std::{error::Error, os::unix::net::UnixStream, time::Duration};
use tokio::{
    io::{unix::AsyncFd, Interest},
    sync::{mpsc, oneshot},
};
use wayland_client::{protocol::wl_registry, Connection, Dispatch, EventQueue, QueueHandle, WEnum};

use gamescope_wayland_client::{
    control::gamescope_control::{
        self, DisplayFlag, DisplaySleepFlags, DisplayTypeFlags, GamescopeControl, ScreenshotFlags,
        ScreenshotType, TargetRefreshCycleFlag,
    },
    input_method::gamescope_input_method_manager::{self, GamescopeInputMethodManager},
};

pub fn screenshot_type_from_u8(value: u8) -> Option<ScreenshotType> {
    match value {
        0 => Some(ScreenshotType::AllRealLayers),
        1 => Some(ScreenshotType::BasePlaneOnly),
        2 => Some(ScreenshotType::FullComposition),
        3 => Some(ScreenshotType::ScreenBuffer),
        _ => None, // Handle invalid values
    }
}

// Enum for internal property update dispatch
#[derive(Debug)]
pub enum WaylandPropertyChanges {
    RefreshRates,
}

/// Enum for internal wayland commands
/// Values starting with Command will be sent from consuming code and processed in the WaylandManager
/// Values starting with Property expect WaylandManager to return a value of given property
#[derive(Debug)]
pub enum WaylandMessage {
    // Command used to take a screenshot
    CommandTakeScreenshot(oneshot::Sender<Result<(), String>>, String, ScreenshotType),
    // Command used to toggle screen sleep
    CommandDisplaySleep(
        oneshot::Sender<Result<(), String>>,
        DisplayTypeFlags,
        DisplaySleepFlags,
    ),
    // Command used to update fps and refresh rate
    CommandSetAppTargetRefreshCycle(
        oneshot::Sender<Result<(), String>>,
        u32,
        TargetRefreshCycleFlag,
    ),
    // Command used to request performance stats for an app
    CommandRequestAppPerformanceStats(oneshot::Sender<Result<u64, String>>, u32),
    // Property list of supported refresh rates
    PropertyRefreshRates(oneshot::Sender<Option<Vec<u32>>>),
}

#[derive(Clone, Debug)]
// Silence unused warnings until we use this struct more
#[allow(dead_code)]
pub struct ActiveDisplay {
    pub connector_name: String,
    pub display_make: String,
    pub display_model: String,
    pub display_flags: WEnum<DisplayFlag>,
    pub refresh_rates: Vec<u32>,
}

// https://github.com/Smithay/wayland-rs/blob/master/wayland-client/examples/simple_window.rs

// This struct represents the state of our app. This simple app does not
// need any state, by this type still supports the `Dispatch` implementations.
pub struct WaylandState {
    property_dispatch_tx: mpsc::Sender<WaylandPropertyChanges>,
    control: Option<GamescopeControl>,
    input_method_manager: Option<GamescopeInputMethodManager>,
    active_display: Option<ActiveDisplay>,
    last_frametime: u64,
}

impl WaylandState {
    fn new(property_dispatch_tx: mpsc::Sender<WaylandPropertyChanges>) -> Self {
        WaylandState {
            property_dispatch_tx,
            control: None,
            input_method_manager: None,
            active_display: None,
            last_frametime: 0,
        }
    }
}

// Implement `Dispatch<WlRegistry, ()> for out state. This provides the logic
// to be able to process events for the wl_registry interface.
//
// The second type parameter is the user-data of our implementation. It is a
// mechanism that allows you to associate a value to each particular Wayland
// object, and allow different dispatching logic depending on the type of the
// associated value.
//
// In this example, we just use () as we don't have any value to associate. See
// the `Dispatch` documentation for more details about this.
impl Dispatch<wl_registry::WlRegistry, ()> for WaylandState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<WaylandState>,
    ) {
        // When receiving events from the wl_registry, we are only interested in the
        // `global` event, which signals a new available global.
        // When receiving this event, we just print its characteristics in this example.
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match &interface[..] {
                "gamescope_control" => {
                    log::debug!("Found gamescope control interface for Wayland!");
                    let control = registry.bind::<GamescopeControl, _, _>(name, version, qh, ());
                    state.control = Some(control);
                }
                "gamescope_input_method_manager" => {
                    let input_method_manager =
                        registry.bind::<GamescopeInputMethodManager, _, _>(name, version, qh, ());
                    state.input_method_manager = Some(input_method_manager);
                }
                _ => {}
            }
        }
    }
}

/// Handle events going to the [GamescopeControl] object.
impl Dispatch<GamescopeControl, ()> for WaylandState {
    fn event(
        state: &mut Self,
        _control: &gamescope_control::GamescopeControl,
        event: gamescope_control::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<WaylandState>,
    ) {
        match event {
            gamescope_control::Event::FeatureSupport {
                feature,
                version,
                flags,
            } => {
                log::debug!("Feature supported: {}, {}, {}", feature, version, flags);
            }
            gamescope_control::Event::ScreenshotTaken { path } => {
                log::info!("Screenshot taken at path: {}", path);
            }
            gamescope_control::Event::ActiveDisplayInfo {
                connector_name,
                display_make,
                display_model,
                display_flags,
                valid_refresh_rates,
            } => {
                // Vec<u32> conversion
                let refresh_rates: Vec<u32> = valid_refresh_rates
                    .chunks_exact(4)
                    .map(|b| u32::from_le_bytes(b.try_into().unwrap()))
                    .collect();
                log::info!(
                    "Active display: connector: {} display: {} {} flags: {:?} refresh_rates: {:?}",
                    connector_name,
                    display_make,
                    display_model,
                    display_flags,
                    refresh_rates
                );

                state.active_display = Some(ActiveDisplay {
                    connector_name,
                    display_make,
                    display_model,
                    display_flags,
                    refresh_rates,
                });

                if let Err(err) = state
                    .property_dispatch_tx
                    .try_send(WaylandPropertyChanges::RefreshRates)
                {
                    log::error!("Failed to send property dispatch event {err:?}");
                }
            }
            gamescope_control::Event::AppPerformanceStats {
                app_id: _,
                frametime_ns_lo,
                frametime_ns_hi,
            } => {
                let frametime_lo = frametime_ns_lo as u64;
                let frametime_hi = frametime_ns_hi as u64;
                state.last_frametime = frametime_lo | (frametime_hi << 32)
            }
            _ => {}
        }
    }
}

/// Handle events going to the [GamescopeInputMethodManager] object.
impl Dispatch<GamescopeInputMethodManager, ()> for WaylandState {
    fn event(
        _state: &mut Self,
        _control: &gamescope_input_method_manager::GamescopeInputMethodManager,
        _event: gamescope_input_method_manager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<WaylandState>,
    ) {
    }
}

pub struct WaylandManager {
    command_tx: mpsc::Sender<WaylandMessage>,
    socket_path: String,
    pub property_dispatch_rx: Option<mpsc::Receiver<WaylandPropertyChanges>>,
}

impl WaylandManager {
    pub async fn new(socket_path: String) -> Result<Self, Box<dyn Error>> {
        let (command_tx, command_rx) = tokio::sync::mpsc::channel(64);
        let (property_dispatch_tx, property_dispatch_rx) = tokio::sync::mpsc::channel(64);

        let instance = Self {
            command_tx,
            property_dispatch_rx: Some(property_dispatch_rx),
            socket_path,
        };

        instance.run(command_rx, property_dispatch_tx).await?;

        Ok(instance)
    }

    async fn run(
        &self,
        mut command_rx: mpsc::Receiver<WaylandMessage>,
        property_dispatch_tx: mpsc::Sender<WaylandPropertyChanges>,
    ) -> Result<(), Box<dyn Error>> {
        let stream = UnixStream::connect(&self.socket_path)?;
        let conn = wayland_client::Connection::from_socket(stream)?;

        log::info!("Connected to wayland display on: {}", self.socket_path);

        // Retrieve the WlDisplay Wayland object from the connection. This object is
        // the starting point of any Wayland program, from which all other objects will
        // be created.
        let display = conn.display();

        log::debug!("Got wayland display: {:?}", display);

        // Create an event queue for our event processing
        let mut event_queue = conn.new_event_queue();
        // An get its handle to associated new objects to it
        let qh = event_queue.handle();

        // Create a wl_registry object by sending the wl_display.get_registry request
        // This method takes two arguments: a handle to the queue the newly created
        // wl_registry will be assigned to, and the user-data that should be associated
        // with this registry (here it is () as we don't need user-data).
        let _registry = display.get_registry(&qh, ());

        // Create state for the application
        let mut state = WaylandState::new(property_dispatch_tx);

        // To actually receive the events, we invoke the `sync_roundtrip` method. This method
        // is special and you will generally only invoke it during the setup of your program:
        // it will block until the server has received and processed all the messages you've
        // sent up to now.
        //
        // In our case, that means it'll block until the server has received our
        // wl_display.get_registry request, and as a reaction has sent us a batch of
        // wl_registry.global events.
        //
        // `sync_roundtrip` will then empty the internal buffer of the queue it has been invoked
        // on, and thus invoke our `Dispatch` implementation that prints the list of advertized
        // globals.
        event_queue.roundtrip(&mut state)?;

        // Get initial Wayland result to assign control and input manager
        let result = event_queue.blocking_dispatch(&mut state)?;
        log::debug!("Initial wayland result: {result}, test:{:?}", conn.flush());

        let socket_path = self.socket_path.clone();

        // Run loop to listen for commands
        tokio::task::spawn(async move {
            loop {
                // Unwrap is safe as long as we dont use event_queue read on any other threads (we dont as of writing)
                let read_guard = event_queue.prepare_read().unwrap();
                let async_fd =
                    AsyncFd::with_interest(read_guard.connection_fd(), Interest::READABLE)
                        .expect("AsyncFd failed");

                tokio::select! {
                    // Use AsyncFd uses epoll on Linux
                    // We do so to get notified when new content is available in the socket
                    Ok(_) = async_fd.readable() => {
                        log::trace!("Wayland socket is readable");
                        // Asyncfd depends on read guard, we need to drop it first
                        drop(async_fd);
                        read_guard.read().ok();
                        event_queue.dispatch_pending(&mut state).ok();
                    }

                    message = command_rx.recv() => {
                        // Asyncfd depends on read guard, we need to drop it first
                        drop(async_fd);
                        // Drop read guard since command handlers may want to do reading too
                        drop(read_guard);

                        let Some(message) = message else { break };
                        log::debug!("Wayland Message: {:?}", message);

                        if let Err(err) = Self::handle_dbus_message(&conn, &mut event_queue, &mut state, message).await {
                            log::error!("Error processing wayland message: err:{err:?}");
                        }
                    }
                }
            }

            log::info!("Finished listening to wayland path: {socket_path}");
        });

        Ok(())
    }

    async fn handle_dbus_message(
        conn: &Connection,
        event_queue: &mut EventQueue<WaylandState>,
        state: &mut WaylandState,
        message: WaylandMessage,
    ) -> Result<(), Box<dyn Error>> {
        match message {
            WaylandMessage::CommandTakeScreenshot(tx, file_path, screenshot_type) => {
                let res = Self::use_state(state, async |state| {
                    log::info!(
                        "Taking screenshot of type: {screenshot_type:?} and saving to {file_path}"
                    );

                    state.control.as_ref().unwrap().take_screenshot(
                        file_path,
                        screenshot_type,
                        ScreenshotFlags::Dummy,
                    );
                    Self::dispatch(&conn, event_queue, state).await?;

                    Ok(())
                })
                .await;

                if let Err(_) = tx.send(res) {
                    log::error!("Error sending response back during [WaylandMessage::CommandTakeScreenshot]");
                }
            }

            WaylandMessage::CommandDisplaySleep(tx, display_type, flags) => {
                let res = Self::use_state(state, async |state| {
                    state
                        .control
                        .as_ref()
                        .unwrap()
                        .display_sleep(display_type, flags);
                    conn.flush().ok();
                    Ok(())
                })
                .await;

                if let Err(_) = tx.send(res) {
                    log::error!(
                        "Error sending response back during [WaylandMessage::CommandDisplaySleep]"
                    );
                }
            }

            WaylandMessage::CommandSetAppTargetRefreshCycle(tx, fps, flags) => {
                let res = Self::use_state(state, async |state| {
                    state
                        .control
                        .as_ref()
                        .unwrap()
                        .set_app_target_refresh_cycle(fps, flags);
                    conn.flush().ok();
                    Ok(())
                })
                .await;

                if let Err(_) = tx.send(res) {
                    log::error!("Error sending response back during [WaylandMessage::CommandSetAppTargetRefreshCycle]");
                }
            }

            WaylandMessage::CommandRequestAppPerformanceStats(tx, app_id) => {
                let res = Self::use_state(state, async |state| {
                    state
                        .control
                        .as_ref()
                        .unwrap()
                        .request_app_performance_stats(app_id);
                    Self::dispatch(&conn, event_queue, state).await?;
                    Ok(())
                })
                .await;

                if let Err(_) = tx.send(res.map(|()| state.last_frametime)) {
                    log::error!("Error sending response back during [WaylandMessage::CommandSetAppTargetRefreshCycle]");
                }
            }

            WaylandMessage::PropertyRefreshRates(tx) => {
                let res = state
                    .active_display
                    .as_ref()
                    .map(|disp| disp.refresh_rates.clone());
                if let Err(_) = tx.send(res) {
                    log::error!(
                        "Error sending response back during [WaylandMessage::PropertyRefreshRates]"
                    );
                }
            }
        }

        Ok(())
    }

    async fn use_state<F>(state: &mut WaylandState, callback: F) -> Result<(), String>
    where
        F: AsyncFnOnce(&mut WaylandState) -> Result<(), String>,
    {
        if state.control.is_none() {
            return Err("No control found".to_owned());
        }

        callback(state).await?;

        Ok(())
    }

    async fn dispatch(
        conn: &Connection,
        event_queue: &mut EventQueue<WaylandState>,
        state: &mut WaylandState,
    ) -> Result<usize, String> {
        conn.flush().map_err(|err| {
            log::error!("Could not flush wayland queue, err:{err:?}");
            err.to_string()
        })?;

        // Wait for readable signal or until a timeout
        {
            let read_guard = event_queue.prepare_read().unwrap();
            let async_fd = AsyncFd::with_interest(read_guard.connection_fd(), Interest::READABLE)
                .expect("AsyncFd failed");
            let timeout_result =
                tokio::time::timeout(Duration::from_millis(100), async_fd.readable()).await;
            if timeout_result.is_err() {
                return Err("No response from gamescope".to_owned());
            }
            drop(async_fd);
            read_guard.read().ok();
        }

        event_queue.dispatch_pending(state).map_err(|err| {
            log::error!("Could not dispatch pending events, err:{err:?}");
            err.to_string()
        })
    }

    pub async fn send(&self, msg: WaylandMessage) -> Result<(), Box<dyn Error>> {
        Ok(self.command_tx.send(msg).await?)
    }
}
