use nix::unistd::Uid;
use std::{env, error::Error, os::unix::net::UnixStream, sync::Arc};
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex,
};
use tokio_stream::{wrappers::ReceiverStream, StreamExt};
use wayland_client::{protocol::wl_registry, Connection, Dispatch, QueueHandle};

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
/// Values starting with Rpc are sent from WaylandManager and processed by consuming code
#[derive(Clone, Debug)]
pub enum WaylandMessage {
    CommandTakeScreenshot(Sender<Result<(), String>>, String, ScreenshotType),
    RpcRegisterControl(GamescopeControl),
    RpcRegisterInputMethodManager(GamescopeInputMethodManager),
    RpcTerminate,
}

// https://github.com/Smithay/wayland-rs/blob/master/wayland-client/examples/simple_window.rs

// This struct represents the state of our app. This simple app does not
// need any state, by this type still supports the `Dispatch` implementations.
pub struct WaylandState {
    pub rpc_tx: Sender<WaylandMessage>,
    pub running: bool,
}

impl WaylandState {
    fn new(rpc_tx: Sender<WaylandMessage>) -> Self {
        WaylandState {
            rpc_tx,
            running: true,
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

                    let tx = state.rpc_tx.clone();
                    tokio::spawn(async move {
                        if let Err(err) = tx.send(WaylandMessage::RpcRegisterControl(control)).await
                        {
                            log::error!(
                                "Error sending WaylandMessage::RpcRegisterControl, err:{err:?}"
                            );
                        }
                    });
                }
                "gamescope_input_method_manager" => {
                    let input_method_manager =
                        registry.bind::<GamescopeInputMethodManager, _, _>(name, version, qh, ());

                    let tx = state.rpc_tx.clone();
                    tokio::spawn(async move {
                        if let Err(err) = tx
                            .send(WaylandMessage::RpcRegisterInputMethodManager(
                                input_method_manager,
                            ))
                            .await
                        {
                            log::error!(
                                "Error sending WaylandMessage::RpcRegisterInputMethodManager, err:{err:?}"
                            );
                        }
                    });
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
    control: Arc<Mutex<Option<GamescopeControl>>>,
    input_method_manager: Arc<Mutex<Option<GamescopeInputMethodManager>>>,
    command_tx: Sender<WaylandMessage>,
}

impl WaylandManager {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let (command_tx, command_rx) = tokio::sync::mpsc::channel::<WaylandMessage>(64);
        let instance = Self {
            control: Arc::default(),
            input_method_manager: Arc::default(),
            command_tx,
        };

        instance.run(command_rx).await?;

        Ok(instance)
    }

    async fn run(&self, command_rx: Receiver<WaylandMessage>) -> Result<(), Box<dyn Error>> {
        let runtime_dir =
            env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| format!("/run/user/{}", Uid::current()));
        let socket_path = format!("{}/gamescope-0", runtime_dir);

        let stream = UnixStream::connect(&socket_path)?;
        let conn = wayland_client::Connection::from_socket(stream)?;

        log::info!("Connected to wayland display on: {}", socket_path);

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
        let (rpc_tx, rpc_rx) = tokio::sync::mpsc::channel::<WaylandMessage>(64);
        let mut state = WaylandState::new(rpc_tx.clone());

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

        // Run the event loop in a separate thread
        tokio::task::spawn_blocking(move || {
            while state.running {
                let result = event_queue.blocking_dispatch(&mut state);

                log::debug!("Got Wayland result: {:?}", result);

                if let Err(err) = result {
                    log::error!(
                        "Got error result when processing wayland event queue, err:{err:?}"
                    );
                    state.running = false;
                }
            }

            if let Err(err) = rpc_tx.blocking_send(WaylandMessage::RpcTerminate) {
                log::error!("Error sending WaylandMessage::RpcTerminate, err:{err:?}");
            }
        });

        // Wrap channels into streams
        let rpc_stream = ReceiverStream::new(rpc_rx);
        let command_stream = ReceiverStream::new(command_rx);

        // Clone state so we can modify it in the stream loop
        let control = self.control.clone();
        let input_method_manager = self.input_method_manager.clone();

        // Run loop to listen for commands and rpcs
        tokio::task::spawn(async move {
            let mut merged_stream = rpc_stream.merge(command_stream);
            while let Some(message) = merged_stream.next().await {
                log::debug!("Wayland Message: {:?}", message);

                let res: Result<(), Box<dyn Error>> = {
                    match message.clone() {
                        WaylandMessage::RpcTerminate => break,
                        WaylandMessage::RpcRegisterControl(c) => {
                            *control.lock().await = Some(c);
                        }
                        WaylandMessage::RpcRegisterInputMethodManager(imm) => {
                            *input_method_manager.lock().await = Some(imm)
                        }

                        WaylandMessage::CommandTakeScreenshot(tx, file_path, screenshot_type) => {
                            let res = Self::use_control(&control, |control| {
                                log::info!("Taking screenshot of type:{screenshot_type:?} and saving to {file_path}");

                                control.take_screenshot(
                                    file_path,
                                    screenshot_type,
                                    ScreenshotFlags::Dummy,
                                );
                                conn.flush().map_err(|err| {
                                    log::error!("Could not flush wayland queue when taking screenshot, err:{err:?}");
                                    err.to_string()
                                })?;

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
                    log::error!("Error processing wayland message:{message:?}, err:{err:?}");
                }
            }
        });

        Ok(())
    }

    async fn use_control<F>(
        control: &Arc<Mutex<Option<GamescopeControl>>>,
        callback: F,
    ) -> Result<(), String>
    where
        F: FnOnce(&GamescopeControl) -> Result<(), String>,
    {
        let control = control.lock().await;
        let control = control.as_ref().ok_or("No control found")?;

        callback(control)?;

        Ok(())
    }

    pub async fn send(&self, msg: WaylandMessage) -> Result<(), Box<dyn Error>> {
        Ok(self.command_tx.send(msg).await?)
    }
}
