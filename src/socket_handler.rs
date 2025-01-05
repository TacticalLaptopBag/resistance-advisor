use log::{error, warn};

use crate::cons;
use crate::models::{RxSocketMsg, TxSocketMsg};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::process;
use std::sync::mpsc::SendError;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

pub struct SocketHandler {
    sender: mpsc::Sender<TxSocketMsg>,
    pub incognito_allowed: bool,
}

impl SocketHandler {
    // TODO: Everything is being handled statically...
    // This doesn't seem right.
    pub fn new() -> SocketHandler {
        let (sender, receiver) = mpsc::channel();

        thread::spawn(move || {
            let write_lock = Arc::new(Mutex::new(()));
            let mut overwatch = match UnixStream::connect(cons::OVERWATCH_SOCKET_PATH) {
                Ok(sock) => sock,
                Err(e) => {
                    error!("Failed to connect to Overwatch socket: {}", e);
                    process::exit(1);
                }
            };

            let heartbeat_overwatch = match overwatch.try_clone() {
                Ok(sock) => sock,
                Err(e) => {
                    error!("Failed to clone handle to Overwatch socket: {}", e);
                    process::exit(1);
                }
            };
            let heartbeat_write_lock = Arc::clone(&write_lock);

            // Start listening for messages
            thread::spawn(move || {
                Self::incoming_msg_thread(heartbeat_overwatch, heartbeat_write_lock)
            });

            while let Ok(socket_msg) = receiver.recv() {
                Self::send_msg(&socket_msg, &mut overwatch, &write_lock);
            }
        });

        return SocketHandler {
            sender,
            incognito_allowed: false,
        };
    }

    pub fn send(&self, msg: TxSocketMsg) -> Result<(), SendError<TxSocketMsg>> {
        return self.sender.send(msg);
    }

    fn send_msg(msg: &TxSocketMsg, overwatch: &mut UnixStream, write_lock: &Arc<Mutex<()>>) {
        let msg_str = match serde_json::to_string(&msg) {
            Ok(str) => str,
            Err(e) => {
                error!("Failed to serialize message to string: {}", e);
                process::exit(1);
            }
        };
        let msg_buf = msg_str.into_bytes();
        let msg_len_bytes = crate::serialize_length(msg_buf.len());

        // Get a lock on writing to Overwatch
        let _guard = match write_lock.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        match overwatch
            .write_all(&msg_len_bytes)
            .and(overwatch.write_all(&msg_buf).and(overwatch.flush()))
        {
            Ok(_) => (),
            Err(e) => {
                error!("Failed to write message to Overwatch socket: {}", e);
                process::exit(1);
            }
        };
    }

    fn incoming_msg_thread(mut overwatch: UnixStream, write_lock: Arc<Mutex<()>>) {
        loop {
            let msg_len = match crate::deserialize_length(&mut overwatch) {
                Ok(len) => len,
                Err(e) => {
                    error!("Failed to deserialize socket message length: {}", e);
                    process::exit(1);
                }
            };

            let mut msg_buf = vec![0u8; msg_len];
            match overwatch.read_exact(&mut msg_buf) {
                Ok(_) => (),
                Err(e) => {
                    error!("Failed to read socket message: {}", e);
                    process::exit(1);
                }
            }

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
            let msg: RxSocketMsg = match serde_json::from_str(&msg_str) {
                Ok(msg) => msg,
                Err(e) => {
                    warn!(
                        "Incoming message from Overwatch was in an invalid format: {}",
                        e
                    );
                    continue;
                }
            };
            Self::handle_incoming_msg(msg, &mut overwatch, &write_lock);
        }
    }

    fn handle_incoming_msg(
        msg: RxSocketMsg,
        overwatch: &mut UnixStream,
        write_lock: &Arc<Mutex<()>>,
    ) {
        let response: Option<TxSocketMsg> = match msg {
            RxSocketMsg::Heartbeat {} => Some(TxSocketMsg::Heartbeat { incognito: false }),
        };

        match response {
            Some(msg) => Self::send_msg(&msg, overwatch, write_lock),
            None => (),
        }
    }
}
