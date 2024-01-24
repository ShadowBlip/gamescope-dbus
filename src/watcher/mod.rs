use inotify::{EventMask, Inotify, WatchMask};
use tokio::sync::broadcast::Sender;

#[derive(Debug, Clone)]
pub enum WatchEvent {
    Create {
        name: String,
        mask: EventMask,
        path: String,
    },
    Modify {
        name: String,
        mask: EventMask,
        path: String,
    },
    Delete {
        name: String,
        mask: EventMask,
        path: String,
    },
}

/// Watch for filesystem changes on the given path, sending [WatchEvent]
/// to the given channel.
pub fn watch(path: String, tx: Sender<WatchEvent>) {
    let mut inotify = Inotify::init().expect("Failed to initialize inotify");

    inotify
        .watches()
        .add(path.clone(), WatchMask::CREATE | WatchMask::DELETE)
        .expect("Failed to add inotify watch");

    // Listen for watch events
    let mut buffer = [0u8; 4096];
    // Use the equivalent of a "do-while" loop so the initial value is
    // processed before awaiting the `changed()` future.
    loop {
        let events = inotify
            .read_events_blocking(&mut buffer)
            .expect("Failed to read inotify events");

        for event in events {
            // Send the event over our channel
            log::debug!("inotify: {:?}", event.name.unwrap());
            let name = String::from(event.name.unwrap().to_str().unwrap());

            if event.mask.contains(EventMask::CREATE) {
                let value = WatchEvent::Create {
                    name,
                    mask: event.mask,
                    path: path.clone(),
                };
                log::debug!("Sending create event");
                if let Err(e) = tx.send(value) {
                    log::error!("Error sending event: {}", e)
                }
                log::debug!("Sent create event");
                //if event.mask.contains(EventMask::ISDIR) {
                //    println!("Directory created: {:?}", event.name);
                //} else {
                //    println!("File created: {:?}", event.name);
                //}
            } else if event.mask.contains(EventMask::DELETE) {
                let value = WatchEvent::Delete {
                    name,
                    mask: event.mask,
                    path: path.clone(),
                };
                log::debug!("Sending delete event");
                if let Err(e) = tx.send(value) {
                    log::error!("Error sending event: {}", e)
                }
            } else if event.mask.contains(EventMask::MODIFY) {
                let value = WatchEvent::Modify {
                    name,
                    mask: event.mask,
                    path: path.clone(),
                };
                log::debug!("Sending modify event");
                if let Err(e) = tx.send(value) {
                    log::error!("Error sending event: {}", e)
                }
            }
        }
    }
}
