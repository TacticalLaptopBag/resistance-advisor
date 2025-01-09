use log::{error, warn};

use crate::cons;
use crate::models::{OverwatchMsg, AdvisorMsg};
use std::io::{Read, Write};
use std::ops::Deref;
use std::os::unix::net::UnixStream;
use std::process;
use std::sync::mpsc::SendError;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

pub struct SocketHandler {
    sender: mpsc::Sender<AdvisorMsg>,
    pub incognito_allowed: Arc<Mutex<bool>>,
}

impl SocketHandler {
    // TODO: Everything is being handled statically...
    // This doesn't seem right.
    pub fn new() -> SocketHandler {
        let (sender, receiver) = mpsc::channel();

        let incognito_allowed = Arc::new(Mutex::new(false));
        let incognito_thread = Arc::clone(&incognito_allowed);
        thread::spawn(move || {
            let write_lock = Arc::new(Mutex::new(()));
            let mut overwatch =
                UnixStream::connect(cons::OVERWATCH_SOCKET_PATH).unwrap_or_else(|e| {
                    error!("Failed to connect to Overwatch socket: {}", e);
                    process::exit(1);
                });

            let heartbeat_overwatch = overwatch.try_clone().unwrap_or_else(|e| {
                error!("Failed to clone handle to Overwatch socket: {}", e);
                process::exit(1);
            });
            let heartbeat_write_lock = Arc::clone(&write_lock);

            // Start listening for messages
            thread::spawn(move || {
                Self::incoming_msg_thread(heartbeat_overwatch, heartbeat_write_lock, incognito_thread);
            });

            while let Ok(socket_msg) = receiver.recv() {
                Self::send_msg(&socket_msg, &mut overwatch, &write_lock);
            }
        });

        return SocketHandler {
            sender,
            incognito_allowed,
        };
    }

    pub fn send(&self, msg: AdvisorMsg) -> Result<(), SendError<AdvisorMsg>> {
        return self.sender.send(msg);
    }

    fn send_msg(msg: &AdvisorMsg, overwatch: &mut UnixStream, write_lock: &Arc<Mutex<()>>) {
        let msg_str = serde_json::to_string(&msg).unwrap_or_else(|e| {
            error!("Failed to serialize message to string: {}", e);
            process::exit(1);
        });
        let msg_buf = msg_str.into_bytes();
        let msg_len_bytes = crate::serialize_length(msg_buf.len());

        // Get a lock on writing to Overwatch
        let _guard = write_lock
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        overwatch
            .write_all(&msg_len_bytes)
            .and(overwatch.write_all(&msg_buf).and(overwatch.flush()))
            .unwrap_or_else(|e| {
                error!("Failed to write message to Overwatch socket: {}", e);
                process::exit(1);
            });
    }

    fn incoming_msg_thread(mut overwatch: UnixStream, write_lock: Arc<Mutex<()>>, incognito_allowed: Arc<Mutex<bool>>) {
        loop {
            let msg_len = crate::deserialize_length(&mut overwatch).unwrap_or_else(|e| {
                error!("Failed to deserialize socket message length: {}", e);
                process::exit(1);
            });

            let mut msg_buf = vec![0u8; msg_len];
            overwatch.read_exact(&mut msg_buf).unwrap_or_else(|e| {
                error!("Failed to read socket message: {}", e);
                process::exit(1);
            });

            let msg_str = match String::from_utf8(msg_buf) {
                Ok(str) => str,
                Err(e) => {
                    warn!(
                        "Incoming message from Overwatch was not in UTF-8 format: {}",
                        e
                    );
                    continue;
                }
            };
            let msg: OverwatchMsg = match serde_json::from_str(&msg_str) {
                Ok(msg) => msg,
                Err(e) => {
                    warn!(
                        "Incoming message from Overwatch was in an invalid format: {}",
                        e
                    );
                    continue;
                }
            };
            Self::handle_incoming_msg(msg, &mut overwatch, &write_lock, &incognito_allowed);
        }
    }

    fn handle_incoming_msg(
        msg: OverwatchMsg,
        overwatch: &mut UnixStream,
        write_lock: &Arc<Mutex<()>>,
        incognito_allowed: &Arc<Mutex<bool>>,
    ) {
        let response: Option<AdvisorMsg> = match msg {
            OverwatchMsg::Heartbeat {} => {
                let incognito = incognito_allowed.lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                Some(AdvisorMsg::Heartbeat { incognito: *incognito })
            },
        };

        match response {
            Some(msg) => Self::send_msg(&msg, overwatch, write_lock),
            None => (),
        }
    }
}
