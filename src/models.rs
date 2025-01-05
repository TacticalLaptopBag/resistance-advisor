use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum RxBrowserMsg {
    #[serde(rename = "init")]
    Init { incognito: bool },
    #[serde(rename = "navigation")]
    Navigation { url: String },
}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum TxBrowserMsg {
    #[serde(rename = "ack")]
    Ack {},
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum RxSocketMsg {
    #[serde(rename = "heartbeat")]
    Heartbeat {},
}

#[derive(Serialize, Debug)]
#[serde(tag = "type")]
pub enum TxSocketMsg {
    #[serde(rename = "heartbeat")]
    Heartbeat { incognito: bool },
    #[serde(rename = "navigation")]
    Navigation { url: String },
}
