use crate::models::{RxSocketMsg, TxSocketMsg};
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;
use std::sync::mpsc::SendError;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;
use std::process;

const OVERWATCH_SOCKET_PATH: &str = "/tmp/overwatch";

pub struct SocketHandler {
    sender: mpsc::Sender<TxSocketMsg>,
    pub incognito_allowed: bool,
}

impl SocketHandler {
    pub fn new() -> SocketHandler {
        let (sender, receiver) = mpsc::channel();

        // Claude wrote this, but I'm not sure if it's useful
        // let sender_clone: mpsc::Sender<TxSocketMsg> = sender.clone();

        thread::spawn(move || {
            let write_lock = Arc::new(Mutex::new(()));
            let mut overwatch = UnixStream::connect(OVERWATCH_SOCKET_PATH)
                .expect("Failed to connect to Overwatch socket");

            let heartbeat_overwatch = overwatch.try_clone().unwrap();
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
        let msg_str = serde_json::to_string(&msg).expect("Failed to serialize socket message");
        let msg_buf = msg_str.into_bytes();
        let msg_len_bytes = crate::serialize_length(msg_buf.len());

        // Get a lock on writing to Overwatch
        let _guard = match write_lock.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        };

        overwatch
            .write_all(&msg_len_bytes)
            .expect("Failed to write socket message");
        overwatch
            .write_all(&msg_buf)
            .expect("Failed to write socket message");
        overwatch.flush().expect("Failed to flush socket message");
    }

    fn incoming_msg_thread(mut overwatch: UnixStream, write_lock: Arc<Mutex<()>>) {
        loop {
            // TODO: If Overwatch exits while here, this thread panics
            let msg_len = match crate::deserialize_length(&mut overwatch) {
                Ok(len) => len,
                Err(e) => {
                    eprintln!("Failed to deserialize socket message length: {}", e);
                    process::exit(1);
                },
            };

            let mut msg_buf = vec![0u8; msg_len];
            overwatch
                .read_exact(&mut msg_buf)
                .expect("Failed to read socket message");

            let msg_str =
                String::from_utf8(msg_buf).expect("Failed to read socket message as UTF-8");
            let msg: RxSocketMsg =
                serde_json::from_str(&msg_str).expect("Failed to parse socket message as valid");
            Self::handle_incoming_msg(msg, &mut overwatch, &write_lock);
        }
    }

    fn handle_incoming_msg(
        msg: RxSocketMsg,
        overwatch: &mut UnixStream,
        write_lock: &Arc<Mutex<()>>,
    ) {
        let response: Option<TxSocketMsg> = match msg {
            RxSocketMsg::Heartbeat {} => {
                Some(TxSocketMsg::Heartbeat { 
                    incognito: false,
                })
            }
        };

        match response {
            Some(msg) => 
                Self::send_msg(&msg, overwatch, write_lock),
            None => ()
        }
    }
}
