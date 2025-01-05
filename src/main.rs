mod browser_handler;
mod cons;
mod models;
mod socket_handler;

use std::io::Read;
use std::{io, process};

use browser_handler::BrowserHandler;
use log::LevelFilter;
use simplelog::TermLogger;
use socket_handler::SocketHandler;
use syslog::{BasicLogger, Facility, Formatter3164};

pub fn serialize_length(len: usize) -> [u8; 4] {
    return (len as u32).to_ne_bytes();
}

pub fn deserialize_length<T: Read>(stream: &mut T) -> io::Result<usize> {
    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf)?;
    let len = u32::from_ne_bytes(len_buf) as usize;
    return Ok(len);
}

fn init_logging() {
    // Setup system logging
    let formatter = Formatter3164 {
        facility: Facility::LOG_USER,
        hostname: None,
        process: "resistance-advisor".into(),
        pid: process::id(),
    };

    let syslog_logger = match syslog::unix(formatter) {
        Ok(logger) => logger,
        Err(e) => {
            eprintln!("Failed to connect to syslog: {}", e);
            process::exit(1);
        }
    };

    let syslog_box = Box::new(BasicLogger::new(syslog_logger));

    // Setup terminal logging to stderr
    let term_logger = TermLogger::new(
        LevelFilter::Warn,
        simplelog::Config::default(),
        simplelog::TerminalMode::Stderr,
        simplelog::ColorChoice::Auto,
    );

    match multi_log::MultiLogger::init(vec![syslog_box, term_logger], cons::LOG_LEVEL) {
        Ok(_) => (),
        Err(e) => {
            eprintln!("Failed to initialize loggers: {}", e);
            process::exit(1);
        }
    }
}

fn main() {
    init_logging();

    // TODO: If threads crash, the program persists
    // Using `process::exit` from any thread will successfully terminate all threads
    let socket = SocketHandler::new();
    let browser = BrowserHandler::new(socket);
    browser.thread_handle.join().unwrap();
}
