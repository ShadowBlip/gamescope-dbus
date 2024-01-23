use std::future::pending;

use simple_logger::SimpleLogger;
use zbus::Connection;

mod gamescope;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    SimpleLogger::new().init().unwrap();
    log::info!("Starting Gamescope DBus");

    // Configure the DBus connection
    let connection = Connection::session().await?;

    // Create an instance of Gamescope Manager
    let manager = gamescope::manager::DBusInterface::new(connection.clone());
    manager.add_xwaylands().await?;

    // Serve the Gamescope Manager on DBus
    let manager_path = String::from("/org/shadowblip/Gamescope/Manager");
    connection.object_server().at(manager_path, manager).await?;
    connection.request_name("org.shadowblip.Gamescope").await?;

    // Do other things or go to wait forever
    pending::<()>().await;

    Ok(())
}
