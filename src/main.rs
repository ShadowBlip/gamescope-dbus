use simple_logger::SimpleLogger;
use tokio::signal;
use zbus::{fdo::ObjectManager, Connection};

mod gamescope;
pub mod utils;
mod watcher;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    SimpleLogger::new().init().unwrap();
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    log::info!("Starting Gamescope DBus v{}", VERSION);

    // Configure the DBus connection
    let connection = Connection::session().await?;

    // Create an ObjectManager to signal when objects are added/removed
    let object_manager = ObjectManager {};
    let object_manager_path = String::from("/org/shadowblip/Gamescope");
    connection
        .object_server()
        .at(object_manager_path, object_manager)
        .await?;

    // Create an instance of Gamescope Manager and its DBus interface
    let mut manager = gamescope::manager::Manager::new(connection.clone());
    let manager_dbus = gamescope::manager::DBusInterface::new();
    manager.update_xwaylands().await?;
    manager.update_waylands().await?;

    // Serve the Gamescope Manager interace on DBus
    let manager_path = String::from("/org/shadowblip/Gamescope/Manager");
    connection
        .object_server()
        .at(manager_path, manager_dbus)
        .await?;
    connection.request_name("org.shadowblip.Gamescope").await?;

    // Listen for gamescope instance changes (added/removed)
    manager.watch_xwaylands().await?;
    manager.watch_waylands().await?;

    // Run the manager in its own thread
    tokio::spawn(async move {
        let _ = manager.run().await;
    });

    signal::ctrl_c().await?;
    log::info!("Terminating...");
    drop(connection);
    std::process::exit(0)
}
