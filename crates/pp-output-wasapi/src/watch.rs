//! Device-change notifications (`IMMNotificationClient`), registered on a thread of their
//! own that lives as long as the returned watch.

use std::sync::{Arc, mpsc};
use std::thread::{self, JoinHandle};

use pp_core::{DeviceChange, DeviceWatch, OutputError};
use wasapi::{DeviceEnumerator, DeviceEventCallbacks, DeviceState, Direction, Role};

use crate::backend::init_com;
use crate::convert::map_error;

/// Unregisters the notifications when dropped.
struct Watch {
    stop: Option<mpsc::Sender<()>>,
    thread: Option<JoinHandle<()>>,
}

impl Drop for Watch {
    fn drop(&mut self) {
        // Closing the channel wakes the thread, which unregisters and exits.
        drop(self.stop.take());
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn callbacks(on_change: &Arc<dyn Fn(DeviceChange) + Send + Sync>) -> DeviceEventCallbacks {
    let mut callbacks = DeviceEventCallbacks::new();
    let f = on_change.clone();
    // Console is the role `get_default_device` (and so "System default") follows.
    callbacks.set_default_device_callback(move |direction, role, id| {
        if direction == Direction::Render && role == Role::Console {
            f(DeviceChange::DefaultChanged { id });
        }
    });
    let f = on_change.clone();
    callbacks.set_device_added_callback(move |id| f(DeviceChange::Added { id }));
    let f = on_change.clone();
    callbacks.set_device_removed_callback(move |id| f(DeviceChange::Removed { id }));
    let f = on_change.clone();
    callbacks.set_device_state_callback(move |id, state| {
        f(DeviceChange::StateChanged {
            id,
            active: state == DeviceState::Active,
        })
    });
    callbacks
}

pub fn watch(
    on_change: Box<dyn Fn(DeviceChange) + Send + Sync>,
) -> Result<DeviceWatch, OutputError> {
    let on_change: Arc<dyn Fn(DeviceChange) + Send + Sync> = Arc::from(on_change);
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let (ready_tx, ready_rx) = mpsc::channel();
    let thread = thread::Builder::new()
        .name("pp-devices".into())
        .spawn(move || {
            init_com();
            let registration = DeviceEnumerator::new()
                .and_then(|e| e.register_notification_callback(callbacks(&on_change)));
            match registration {
                Ok(registration) => {
                    let _ = ready_tx.send(Ok(()));
                    let _ = stop_rx.recv();
                    drop(registration);
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(map_error(e)));
                }
            }
        })
        .map_err(|e| OutputError::Other(e.to_string()))?;
    match ready_rx.recv() {
        Ok(Ok(())) => Ok(Box::new(Watch {
            stop: Some(stop_tx),
            thread: Some(thread),
        })),
        Ok(Err(e)) => {
            let _ = thread.join();
            Err(e)
        }
        Err(_) => Err(OutputError::Other(
            "the device watcher stopped unexpectedly".into(),
        )),
    }
}
