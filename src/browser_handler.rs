use std::io;
use std::io::{Read, StdinLock, StdoutLock, Write};
use std::process;
use std::thread::{self, JoinHandle};

use log::{error, warn};

use crate::models::{RxBrowserMsg, TxSocketMsg};
use crate::{models::TxBrowserMsg, socket_handler::SocketHandler};

pub struct BrowserHandler {
    pub thread_handle: JoinHandle<()>,
}

impl BrowserHandler {
    pub fn new(mut socket: SocketHandler) -> BrowserHandler {
        let handle = thread::spawn(move || {
            // Start browser extension communication
            let mut stdin = io::stdin().lock();
            let mut stdout = io::stdout().lock();

            loop {
                let msg = match Self::read_msg(&mut stdin) {
                    Ok(msg) => msg,
                    Err(e) => {
                        warn!(
                            "Incoming message from Overwatch was in an invalid format: {}",
                            e
                        );
                        continue;
                    }
                };
                Self::handle_msg(msg, &mut socket);
                let response = TxBrowserMsg::Ack {};
                Self::send_msg(&mut stdout, &response);
            }
        });

        return BrowserHandler {
            thread_handle: handle,
        };
    }

    fn handle_msg(msg: RxBrowserMsg, socket: &mut SocketHandler) {
        match msg {
            RxBrowserMsg::Init { incognito } => {
                socket.incognito_allowed = incognito;
            }
            RxBrowserMsg::Navigation { url } => {
                socket
                    .send(TxSocketMsg::Navigation { url })
                    .unwrap_or_else(|e| {
                        error!("Failed to relay Navigation message to Overwatch: {}", e);
                    });
            }
        }
    }

    fn read_msg(stdin: &mut StdinLock) -> Result<RxBrowserMsg, serde_json::Error> {
        // Get the length of the incoming message
        let msg_len = crate::deserialize_length(stdin).unwrap_or_else(|e| {
            error!("Failed to read length of message from Scanner: {}", e);
            process::exit(1);
        });

        let mut msg_buf = vec![0u8; msg_len];
        stdin.read_exact(&mut msg_buf).unwrap_or_else(|e| {
            error!("Failed to read incoming message from Scanner: {}", e);
            process::exit(1);
        });

        let msg_str = String::from_utf8(msg_buf).unwrap_or_else(|e| {
            error!(
                "Incoming message from Scanner was not encoded in UTF-8: {}",
                e
            );
            process::exit(1);
        });

        return serde_json::from_str(msg_str.as_str());
    }

    fn send_msg(stdout: &mut StdoutLock, msg: &TxBrowserMsg) {
        let response_str = serde_json::to_string(msg).unwrap_or_else(|e| {
            error!("Failed to serialize message to Scanner: {}", e);
            process::exit(1);
        });
        let response_buf = response_str.into_bytes();
        let response_len = response_buf.len();

        let len_bytes = crate::serialize_length(response_len);
        stdout
            .write_all(&len_bytes)
            .and(stdout.write_all(&response_buf))
            .and(stdout.flush())
            .unwrap_or_else(|e| {
                error!("Failed to write message to Scanner: {}", e);
                process::exit(1);
            });
    }
}
