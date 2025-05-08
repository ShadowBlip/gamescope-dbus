use std::{
    collections::{HashMap, HashSet},
    error::Error,
    time::Duration,
};
use tokio::{
    fs,
    sync::{broadcast, mpsc},
};
use zbus::{fdo, zvariant::ObjectPath, Connection};
use zbus_macros::dbus_interface;

use crate::{
    utils::{get_run_user_dir, is_gamescope_socket_file},
    watcher::{self, WatchEvent},
};

use super::{wayland, xwayland};

#[derive(Debug, Copy, Clone)]
pub enum WatchType {
    X11,
    Wayland,
}

/// Manager commands define all the different ways to interact with [Manager]
/// over a channel. These commands are processed in an asyncronous thread and
/// dispatched as they come in.
#[derive(Debug)]
#[allow(dead_code)]
pub enum Command {
    FilesystemEvent {
        event: WatchEvent,
        watch_type: WatchType,
    },
    XWaylandAdded {
        name: String,
    },
    XWaylandRemoved {
        name: String,
    },
    WaylandAdded {
        path: String,
    },
    WaylandRemoved {
        path: String,
    },
}

/// Gamescope Manager instance
pub struct Manager {
    /// Connection to the DBus bus
    dbus: Connection,
    /// Mapping of XWayland names (":0", ":1") to DBus path ("/org/shadowblip/Gamescope/XWayland0")
    xwaylands: HashMap<String, String>,
    /// List of existing wayland sockets
    waylands: HashSet<String>,
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
            waylands: HashSet::new(),
        }
    }

    /// Starts the wayland manager and adds its dbus interface
    pub async fn start_wayland_manager(&self, path: String) -> Result<(), Box<dyn Error>> {
        let id = path
            .split('-')
            .last()
            .ok_or("Wrong id found in wayland gamescope socket file name")?;
        let dbus_path = format!("/org/shadowblip/Gamescope/Wayland{}", id);
        let interface =
            wayland::dbus::DBusInterface::new(dbus_path.clone(), self.dbus.clone(), path).await?;
        self.dbus
            .object_server()
            .at(dbus_path.clone(), interface)
            .await?;
        log::info!("Initialized wayland manager at path:{dbus_path}");
        Ok(())
    }

    /// Removes the wayland manager and its dbus interface
    pub async fn remove_wayland_manager(&self, path: String) -> Result<(), Box<dyn Error>> {
        let id = path
            .split('-')
            .last()
            .ok_or("Wrong id found in wayland gamescope socket file name")?;
        let dbus_path = format!("/org/shadowblip/Gamescope/Wayland{}", id);
        self.dbus
            .object_server()
            .remove::<wayland::dbus::DBusInterface, String>(dbus_path.clone())
            .await?;
        log::info!("Removed wayland manager at path:{dbus_path}");
        Ok(())
    }

    async fn start_wayland_manager_for_path(&mut self, path: String) {
        if self.waylands.contains(&path) {
            return;
        }

        if let Err(err) = self.start_wayland_manager(path.clone()).await {
            log::error!("Error starting wayland manager and interface at path:{path}, err:{err:?}");
            return;
        }

        self.waylands.insert(path);
    }

    async fn remove_wayland_manager_for_path(&mut self, path: String) {
        if let Err(err) = self.remove_wayland_manager(path.clone()).await {
            log::error!("Error removing wayland manager at path:{path}, err:{err:?}");
        }

        if let Some((name, _)) = self.xwaylands.iter().find(|x| *x.1 == path) {
            self.waylands.remove(name);
        }
    }

    /// Starts listening for [Command] messages to be sent from clients and
    /// dispatch those events.
    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        log::debug!("Starting manager");
        while let Some(cmd) = self.rx.recv().await {
            log::debug!("Received command: {:?}", cmd);
            match cmd {
                Command::FilesystemEvent { event, watch_type } => {
                    self.on_watch_event(event, watch_type).await;
                }
                Command::XWaylandAdded { name: _ } => self.update_xwaylands().await?,
                Command::XWaylandRemoved { name } => self.remove_xwayland(name).await?,
                Command::WaylandAdded { path } => {
                    if !self.waylands.contains(&path) {
                        // Added sleep here because sometimes we get errors regarding broken IO connection due to starting it too fast when gamescope is restarted
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        self.start_wayland_manager_for_path(path).await;
                    }
                }
                Command::WaylandRemoved { path } => {
                    if self.waylands.contains(&path) {
                        self.remove_wayland_manager_for_path(path).await;
                    }
                }
            }
        }
        log::warn!("Stopping manager");
        Ok(())
    }

    /// Executed when a filesystem watch event occurs
    async fn on_watch_event(&mut self, event: WatchEvent, watch_type: WatchType) {
        log::debug!("Got watch event: {:?}", event);
        match event {
            WatchEvent::Create {
                name,
                mask: _,
                path,
            } => {
                log::info!("Got create event: {}", name);

                match watch_type {
                    WatchType::Wayland => self.on_wayland_create(name, path).await,
                    WatchType::X11 => self.on_x11_create(name).await,
                };
            }
            WatchEvent::Delete {
                name,
                mask: _,
                path,
            } => {
                log::info!("Got delete event: {}", name);

                match watch_type {
                    WatchType::Wayland => self.on_wayland_delete(name, path).await,
                    WatchType::X11 => self.on_x11_delete(name).await,
                };
            }
            _ => (),
        }
    }

    async fn on_x11_create(&self, name: String) {
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

    async fn on_x11_delete(&self, name: String) {
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

    async fn on_wayland_create(&self, name: String, path: String) {
        if !is_gamescope_socket_file(&name) {
            return;
        }

        let _ = self
            .tx
            .send(Command::WaylandAdded {
                path: format!("{path}/{name}"),
            })
            .await;
    }

    async fn on_wayland_delete(&self, name: String, path: String) {
        if !is_gamescope_socket_file(&name) {
            return;
        }

        let _ = self
            .tx
            .send(Command::WaylandRemoved {
                path: format!("{path}/{name}"),
            })
            .await;
    }

    /// Watches for new wayland instances to start and adds them.
    pub async fn watch_waylands(&self) -> Result<(), Box<dyn Error>> {
        self.watch_paths(get_run_user_dir(), WatchType::Wayland)
            .await
    }

    /// Watches for new xwayland instances to start and adds them.
    pub async fn watch_xwaylands(&self) -> Result<(), Box<dyn Error>> {
        self.watch_paths("/tmp/.X11-unix".to_string(), WatchType::X11)
            .await
    }

    /// Watches paths and triggers events.
    async fn watch_paths(&self, path: String, watch_type: WatchType) -> Result<(), Box<dyn Error>> {
        // Create a watch channel for filesystem events
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
                let result = manager_tx
                    .send(Command::FilesystemEvent { event, watch_type })
                    .await;
                if result.is_err() {
                    log::warn!("Failed to send command: {:?}", result);
                }
            }
        });

        // Start watching for filesystem events
        std::thread::spawn(move || {
            log::debug!("Starting filesystem watch on: {}", path);
            watcher::watch(path, watcher_tx);
        });

        Ok(())
    }

    pub async fn remove_xwayland(&mut self, name: String) -> Result<(), Box<dyn Error>> {
        let Some((name, dbus_path)) = self
            .xwaylands
            .iter()
            .find(|x| *x.0 == name)
            .map(|x| (x.0.clone(), x.1.clone()))
        else {
            log::warn!("Skipping remove since xwayland:{name} isn't managed");
            return Ok(());
        };

        log::info!("XWayland was removed: {}", name);
        let path = ObjectPath::from_string_unchecked(dbus_path.to_owned());
        self.dbus
            .object_server()
            .remove::<xwayland::DBusInterface, ObjectPath>(path.clone())
            .await?;
        let _ = self
            .dbus
            .object_server()
            .remove::<xwayland::DBusInterfacePrimary, ObjectPath>(path)
            .await;

        self.xwaylands.remove(&name);

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
        for (name, _) in self.xwaylands.iter() {
            if current_xwaylands.contains(name) {
                log::debug!("XWayland still exists for {}. Skipping.", name);
                continue;
            }

            to_remove.push(name.clone());
        }
        for name in to_remove {
            self.remove_xwayland(name).await?;
        }

        // Create any xwaylands that don't exist
        for (i, name) in current_xwaylands.into_iter().enumerate() {
            if self.xwaylands.contains_key(&name) {
                log::debug!("XWayland is already managed for {}. Skipping.", name);
                continue;
            }

            // Create a new DBus interface to the xwayland instance
            let path = format!("/org/shadowblip/Gamescope/XWayland{}", i);
            let instance =
                xwayland::DBusInterface::new(name.clone(), path.clone(), self.dbus.clone())?;
            let is_primary = instance.primary().await?;

            // Listen for new windows lifecycle
            let window_lifecycle_rx = instance.listen_for_window_lifecycle()?;
            // Propagate gamescope changes to DBus signals
            xwayland::dispatch_window_lifecycle(
                self.dbus.clone(),
                path.clone(),
                window_lifecycle_rx,
                is_primary,
            )
            .await?;

            // Check to see if this is a primary xwayland instance. If it is,
            // also attach the dbus interface with extra methods
            if is_primary {
                log::debug!("Discovered XWayland {} is primary", name);

                // Property changes events
                let primary = xwayland::DBusInterfacePrimary::new(
                    name.clone(),
                    path.clone(),
                    self.dbus.clone(),
                )?;
                let property_changes_rx = primary.listen_for_property_changes()?;
                #[allow(deprecated)]
                let window_created_rx = primary.listen_for_window_created()?;
                self.dbus.object_server().at(path.clone(), primary).await?;

                // Propagate gamescope changes to DBus signals
                xwayland::dispatch_primary_property_changes(
                    self.dbus.clone(),
                    path.clone(),
                    property_changes_rx,
                )
                .await?;

                // Propagate gamescope changes to DBus signals
                #[allow(deprecated)]
                xwayland::dispatch_primary_window_created(
                    self.dbus.clone(),
                    path.clone(),
                    window_created_rx,
                )
                .await?;
            }

            self.dbus.object_server().at(path.clone(), instance).await?;

            // Add the instance
            self.xwaylands.insert(name, path);
        }

        log::info!("Managed XWaylands: {:?}", self.xwaylands);

        Ok(())
    }

    /// Discovers and adds/removes wayland interfaces
    pub async fn update_waylands(&mut self) -> Result<(), Box<dyn Error>> {
        let path = get_run_user_dir();
        let mut dir = fs::read_dir(path).await?;
        let mut current_waylands = HashSet::new();

        while let Ok(Some(folder)) = dir.next_entry().await {
            let name = folder.file_name().to_string_lossy().to_string();
            if !is_gamescope_socket_file(&name) {
                continue;
            }

            let path = folder.path().to_string_lossy().to_string();
            current_waylands.insert(path);
        }

        // Remove
        for wayland in self.waylands.clone() {
            if current_waylands.contains(&wayland) {
                continue;
            }

            self.remove_wayland_manager_for_path(wayland).await;
        }

        // Add
        for path in current_waylands {
            if self.waylands.contains(&path) {
                continue;
            }

            // Start new wayland dbus interface
            self.start_wayland_manager_for_path(path).await;
        }

        log::info!("Managed Waylands: {:?}", self.waylands);

        Ok(())
    }
}

/// DBus interface imeplementation for Gamescope Manager instance
pub struct DBusInterface {
    //// Connection to the DBus bus
    //dbus: Connection,
}

impl DBusInterface {
    /// Returns a new instance of the XWayland DBus interface. Will error if
    /// it cannot establish a connection.
    pub fn new() -> DBusInterface {
        DBusInterface {}
    }
}

#[dbus_interface(name = "org.shadowblip.Gamescope.Manager")]
impl DBusInterface {
    #[dbus_interface(property)]
    async fn name(&self) -> fdo::Result<String> {
        Ok("Manager".into())
    }
}
