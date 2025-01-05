use std::io;
use std::io::{Read, Write, StdinLock, StdoutLock};
use std::thread::{self, JoinHandle};
use std::process;

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
                let msg = Self::read_msg(&mut stdin);
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
            },
            RxBrowserMsg::Navigation { url } => {
                socket.send(TxSocketMsg::Navigation { 
                    url
                }).expect("Failed to relay Navigation message");
            },
        }
    }

    fn read_msg(stdin: &mut StdinLock) -> RxBrowserMsg {
        // Get the length of the incoming message
        let msg_len = match crate::deserialize_length(stdin) {
            Ok(len) => len,
            Err(e) => {
                eprintln!("Failed to read length of message from extension: {}", e);
                process::exit(1);
            }
        };

        let mut msg_buf = vec![0u8; msg_len];
        let _ = stdin.read_exact(&mut msg_buf);

        let msg_str = String::from_utf8(msg_buf).unwrap();

        return serde_json::from_str(msg_str.as_str()).unwrap();
    }

    fn send_msg(stdout: &mut StdoutLock, msg: &TxBrowserMsg) {
        // let response_buf = msg.dump().into_bytes();
        let response_buf = serde_json::to_string(msg).unwrap().into_bytes();
        let response_len = response_buf.len();

        let len_bytes = crate::serialize_length(response_len);
        let _ = stdout.write_all(&len_bytes);
        let _ = stdout.write_all(&response_buf);
        let _ = stdout.flush();
    }
}
