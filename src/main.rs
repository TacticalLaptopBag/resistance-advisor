mod models;
mod socket_handler;
mod browser_handler;

use std::io;
use std::io::Read;

use socket_handler::SocketHandler;
use browser_handler::BrowserHandler;

pub fn serialize_length(len: usize) -> [u8; 4] {
    return (len as u32).to_ne_bytes();
}

pub fn deserialize_length<T: Read>(stream: &mut T) -> io::Result<usize> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let len = u32::from_ne_bytes(len_buf) as usize;
    return Ok(len);
}


fn main() {
    // TODO: If threads crash, the program persists
    // Using `process::exit` from any thread will successfully terminate all threads
    let socket = SocketHandler::new();
    let browser = BrowserHandler::new(socket);
    browser.thread_handle.join().unwrap();
}
