use std::{error::Error, os::unix::net::UnixStream};
use tokio::sync::mpsc::{Receiver, Sender};
use wayland_client::{protocol::wl_registry, Connection, Dispatch, EventQueue, QueueHandle};

use gamescope_wayland_client::{
    control::gamescope_control::{self, GamescopeControl, ScreenshotFlags, ScreenshotType},
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

/// Enum for internal wayland commands
/// Values starting with Command will be sent from consuming code and processed in the WaylandManager
#[derive(Clone, Debug)]
pub enum WaylandMessage {
    // Command used to take a screenshot
    CommandTakeScreenshot(Sender<Result<(), String>>, String, ScreenshotType),
}

// https://github.com/Smithay/wayland-rs/blob/master/wayland-client/examples/simple_window.rs

// This struct represents the state of our app. This simple app does not
// need any state, by this type still supports the `Dispatch` implementations.
pub struct WaylandState {
    control: Option<GamescopeControl>,
    input_method_manager: Option<GamescopeInputMethodManager>,
}

impl WaylandState {
    fn new() -> Self {
        WaylandState {
            control: None,
            input_method_manager: None,
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
        _state: &mut Self,
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
    command_tx: Sender<WaylandMessage>,
    socket_path: String,
}

impl WaylandManager {
    pub async fn new(socket_path: String) -> Result<Self, Box<dyn Error>> {
        let (command_tx, command_rx) = tokio::sync::mpsc::channel::<WaylandMessage>(64);
        let instance = Self {
            command_tx,
            socket_path,
        };

        instance.run(command_rx).await?;

        Ok(instance)
    }

    async fn run(&self, mut command_rx: Receiver<WaylandMessage>) -> Result<(), Box<dyn Error>> {
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
        let mut state = WaylandState::new();

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
            while let Some(message) = command_rx.recv().await {
                log::debug!("Wayland Message: {:?}", message);

                let res: Result<(), Box<dyn Error>> = {
                    match message.clone() {
                        WaylandMessage::CommandTakeScreenshot(tx, file_path, screenshot_type) => {
                            let res = Self::use_state(&mut state, |state| {
                                log::info!("Taking screenshot of type: {screenshot_type:?} and saving to {file_path}");

                                state.control.as_ref().unwrap().take_screenshot(
                                    file_path,
                                    screenshot_type,
                                    ScreenshotFlags::Dummy,
                                );
                                Self::dispatch(&conn, &mut event_queue, state)?;

                                Ok(())
                            })
                            .await;

                            if let Err(err) = tx.send(res).await {
                                log::error!("Error sending response back during [WaylandMessage::CommandTakeScreenshot], err:{err:?}");
                            }
                        }
                    }

                    Ok(())
                };

                if let Err(err) = res {
                    log::error!("Error processing wayland message: {message:?}, err:{err:?}");
                }
            }

            log::info!("Finished listening to wayland path: {socket_path}");
        });

        Ok(())
    }

    async fn use_state<F>(state: &mut WaylandState, callback: F) -> Result<(), String>
    where
        F: FnOnce(&mut WaylandState) -> Result<(), String>,
    {
        if state.control.is_none() {
            return Err("No control found".to_owned());
        }

        callback(state)?;

        Ok(())
    }

    fn dispatch(
        conn: &Connection,
        event_queue: &mut EventQueue<WaylandState>,
        state: &mut WaylandState,
    ) -> Result<usize, String> {
        conn.flush().map_err(|err| {
            log::error!("Could not flush wayland queue, err:{err:?}");
            err.to_string()
        })?;
        event_queue.blocking_dispatch(state).map_err(|err| {
            log::error!("Could not dispatch pending events, err:{err:?}");
            err.to_string()
        })
    }

    pub async fn send(&self, msg: WaylandMessage) -> Result<(), Box<dyn Error>> {
        Ok(self.command_tx.send(msg).await?)
    }
}
