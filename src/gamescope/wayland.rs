use std::os::unix::net::UnixStream;
use wayland_client::{protocol::wl_registry, Connection, Dispatch, QueueHandle};

use gamescope_wayland_client::{
    control::gamescope_control::{self, GamescopeControl, ScreenshotFlags, ScreenshotType},
    input_method::gamescope_input_method_manager::{self, GamescopeInputMethodManager},
};

// https://github.com/Smithay/wayland-rs/blob/master/wayland-client/examples/simple_window.rs

// This struct represents the state of our app. This simple app does not
// need any state, by this type still supports the `Dispatch` implementations.
struct AppData {
    pub control: Option<GamescopeControl>,
    pub input_method_manager: Option<GamescopeInputMethodManager>,
    pub running: bool,
}

impl AppData {
    fn new() -> Self {
        AppData {
            control: None,
            input_method_manager: None,
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
impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<AppData>,
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
                    println!("Found gamescope control interface!");
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
            println!("[{}] {} (v{})", name, interface, version);
        }
    }
}

/// Handle events going to the [GamescopeControl] object.
impl Dispatch<GamescopeControl, ()> for AppData {
    fn event(
        _state: &mut Self,
        _control: &gamescope_control::GamescopeControl,
        event: gamescope_control::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        match event {
            gamescope_control::Event::FeatureSupport {
                feature,
                version,
                flags,
            } => {
                println!("Feature supported: {}, {}, {}", feature, version, flags);
            }
            gamescope_control::Event::ScreenshotTaken { path } => {
                println!("Screenshot taken at path: {}", path);
            }
            _ => {}
        }
    }
}

/// Handle events going to the [GamescopeInputMethodManager] object.
impl Dispatch<GamescopeInputMethodManager, ()> for AppData {
    fn event(
        _state: &mut Self,
        _control: &gamescope_input_method_manager::GamescopeInputMethodManager,
        _event: gamescope_input_method_manager::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
    }
}

fn main() {
    println!("Hello, gamescope!");
    let socket_path = "/run/user/1000/gamescope-0";

    let stream = UnixStream::connect(socket_path).unwrap();
    let conn = wayland_client::Connection::from_socket(stream);
    let conn = conn.unwrap();

    println!("Connected to wayland display on: {}", socket_path);

    // Retrieve the WlDisplay Wayland object from the connection. This object is
    // the starting point of any Wayland program, from which all other objects will
    // be created.
    let display = conn.display();

    println!("Got wayland display: {:?}", display);

    // Create an event queue for our event processing
    let mut event_queue = conn.new_event_queue();
    // An get its handle to associated new objects to it
    let qh = event_queue.handle();

    // Create a wl_registry object by sending the wl_display.get_registry request
    // This method takes two arguments: a handle to the queue the newly created
    // wl_registry will be assigned to, and the user-data that should be associated
    // with this registry (here it is () as we don't need user-data).
    let _registry = display.get_registry(&qh, ());

    // At this point everything is ready, and we just need to wait to receive the events
    // from the wl_registry, our callback will print the advertized globals.
    println!("Advertized globals:");

    // Create state for the application
    let mut state = AppData::new();

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
    event_queue.roundtrip(&mut state).unwrap();
    println!("Got control object from globals");

    // Take a screenshot! (only supported in v3)
    //println!("Taking a screenshot");
    //state.control.as_ref().unwrap().take_screenshot(
    //    "/tmp/foo.png".into(),
    //    ScreenshotType::ScreenBuffer,
    //    ScreenshotFlags::Dummy,
    //);
    //println!("Took screenshot!");

    while state.running {
        let result = event_queue.blocking_dispatch(&mut state);
        println!("Got result: {:?}", result);
        if result.is_err() {
            state.running = false;
        }
    }
}
