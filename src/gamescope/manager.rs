use std::{collections::HashMap, error::Error, time::Duration};
use tokio::sync::{broadcast, mpsc};
use zbus::{fdo, zvariant::ObjectPath, Connection};
use zbus_macros::dbus_interface;

use crate::watcher::{self, WatchEvent};

use super::xwayland;

/// Manager commands define all the different ways to interact with [Manager]
/// over a channel. These commands are processed in an asyncronous thread and
/// dispatched as they come in.
#[derive(Debug)]
pub enum Command {
    FilesystemEvent { event: WatchEvent },
    XWaylandAdded { name: String },
    XWaylandRemoved { name: String },
}

/// Gamescope Manager instance
pub struct Manager {
    /// Connection to the DBus bus
    dbus: Connection,
    /// Mapping of XWayland names (":0", ":1") to DBus path ("/org/shadowblip/Gamescope/XWayland0")
    xwaylands: HashMap<String, String>,
    /// The transmit side of the [rx] channel used to send [Command] messages.
    /// This can be cloned to allow child objects to communicate up to the
    /// manager.
    tx: mpsc::Sender<Command>,
    /// The receive side of the channel used to listen for [Command] messages
    /// from other objects.
    rx: mpsc::Receiver<Command>,
}

impl Manager {
    /// Returns a new instance of the XWayland DBus interface. Will error if
    /// it cannot establish a connection.
    pub fn new(conn: Connection) -> Manager {
        let (tx, rx) = mpsc::channel(32);
        Manager {
            dbus: conn,
            tx,
            rx,
            xwaylands: HashMap::new(),
        }
    }

    /// Starts listening for [Command] messages to be sent from clients and
    /// dispatch those events.
    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        log::debug!("Starting manager");
        while let Some(cmd) = self.rx.recv().await {
            log::debug!("Received command: {:?}", cmd);
            match cmd {
                Command::FilesystemEvent { event } => {
                    self.on_watch_event(event).await;
                }
                Command::XWaylandAdded { name: _ } | Command::XWaylandRemoved { name: _ } => {
                    self.update_xwaylands().await?;
                }
            }
        }
        log::warn!("Stopping manager");
        Ok(())
    }

    /// Executed when a filesystem watch event occurs
    pub async fn on_watch_event(&mut self, event: WatchEvent) {
        log::debug!("Got watch event: {:?}", event);
        match event {
            WatchEvent::Create {
                name,
                mask: _,
                path: _,
            } => {
                log::info!("Got create event: {}", name);
                if !name.starts_with('X') {
                    return;
                }
                let suffix = name.strip_prefix('X').unwrap();

                // Skip X11 sockets with weird names
                if suffix.parse::<u64>().is_err() {
                    return;
                }
                let name = format!(":{}", suffix);
                let _ = self.tx.send(Command::XWaylandAdded { name }).await;
            }
            WatchEvent::Delete {
                name,
                mask: _,
                path: _,
            } => {
                log::info!("Got delete event: {}", name);
                if !name.starts_with('X') {
                    return;
                }
                let suffix = name.strip_prefix('X').unwrap();

                // Skip X11 sockets with weird names
                if suffix.parse::<u64>().is_err() {
                    return;
                }
                let name = format!(":{}", suffix);
                let _ = self.tx.send(Command::XWaylandRemoved { name }).await;
            }
            _ => (),
        }
    }

    /// Watches for new xwayland instances to start and adds them.
    pub async fn watch_xwaylands(&self) -> Result<(), Box<dyn Error>> {
        // Create a watch channel for X11 instance filesystem events
        let (watcher_tx, mut watcher_rx) = broadcast::channel(32);

        // Create a copy of the transmitter, so watch events can propagate to
        // the manager when it is running.
        let manager_tx = self.tx.clone();

        // Listen for watch events and dispatch them
        tokio::spawn(async move {
            log::debug!("Filesystem watch dispatcher started");
            loop {
                let event = watcher_rx.recv().await;
                if event.is_err() {
                    log::warn!("Error receiving event: {:?}", event);
                    break;
                }
                let event = event.unwrap();
                log::debug!("Dispatcher received event: {:?}", event);
                let result = manager_tx.send(Command::FilesystemEvent { event }).await;
                if result.is_err() {
                    log::warn!("Failed to send command: {:?}", result);
                }
            }
        });

        // Start watching for filesystem events
        std::thread::spawn(move || {
            let path = "/tmp/.X11-unix".to_string();
            log::debug!("Starting filesystem watch on: {}", path);
            watcher::watch(path, watcher_tx);
        });

        Ok(())
    }

    /// Discovers and adds/removes xwayland interfaces
    pub async fn update_xwaylands(&mut self) -> Result<(), Box<dyn Error>> {
        log::info!("Updating XWaylands");

        // Wait a small amount of time to allow the socket to be available
        // NOTE: Without this, it seems that we cannot discover the X display
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Discover new gamescopes
        let current_xwaylands = gamescope_x11_client::discover_gamescope_displays()?;
        log::debug!("Discovered XWaylands: {:?}", current_xwaylands);

        // Remove any xwaylands that no longer exist
        let mut to_remove: Vec<String> = Vec::new();
        for (name, dbus_path) in self.xwaylands.iter() {
            if current_xwaylands.contains(name) {
                log::debug!("XWayland still exists for {}. Skipping.", name);
                continue;
            }

            log::info!("XWayland was removed: {}", name);
            let path = ObjectPath::from_string_unchecked(dbus_path.clone());
            self.dbus
                .object_server()
                .remove::<xwayland::DBusInterface, ObjectPath>(path.clone())
                .await?;
            let _ = self
                .dbus
                .object_server()
                .remove::<xwayland::DBusInterfacePrimary, ObjectPath>(path)
                .await;
            to_remove.push(name.clone());
        }
        for name in to_remove {
            self.xwaylands.remove(&name);
        }

        // Create any xwaylands that don't exist
        for (i, name) in current_xwaylands.into_iter().enumerate() {
            if self.xwaylands.get(&name).is_some() {
                log::debug!("XWayland is already managed for {}. Skipping.", name);
                continue;
            }

            // Create a new DBus interface to the xwayland instance
            let instance = xwayland::DBusInterface::new(name.clone())?;
            let path = format!("/org/shadowblip/Gamescope/XWayland{}", i);

            // Check to see if this is a primary xwayland instance. If it is,
            // also attach the dbus interface with extra methods
            if instance.primary().await? {
                println!("Gamescope is primary!");
                let primary = xwayland::DBusInterfacePrimary::new(name.clone())?;
                let changes_rx = primary.listen_for_property_changes()?;
                self.dbus.object_server().at(path.clone(), primary).await?;

                // Propagate gamescope changes to DBus signals
                xwayland::dispatch_property_changes(self.dbus.clone(), path.clone(), changes_rx)
                    .await?;
            }

            self.dbus.object_server().at(path.clone(), instance).await?;

            // Add the instance
            self.xwaylands.insert(name, path);
        }

        log::info!("Managed XWaylands: {:?}", self.xwaylands);

        Ok(())
    }
}

/// DBus interface imeplementation for Gamescope Manager instance
pub struct DBusInterface {
    /// Connection to the DBus bus
    dbus: Connection,
}

impl DBusInterface {
    /// Returns a new instance of the XWayland DBus interface. Will error if
    /// it cannot establish a connection.
    pub fn new(conn: Connection) -> DBusInterface {
        DBusInterface { dbus: conn }
    }
}

#[dbus_interface(name = "org.shadowblip.Gamescope.Manager")]
impl DBusInterface {
    #[dbus_interface(property)]
    async fn name(&self) -> fdo::Result<String> {
        Ok("Manager".into())
    }
}
