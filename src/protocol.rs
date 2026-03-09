use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "snake_case")]
pub enum Request {
    CounterNew {
        name: String,
        step: Option<i64>,
        initial: Option<i64>,
    },
    CounterRead {
        name: String,
    },
    CounterSeek {
        name: String,
    },
    CounterRm {
        name: String,
    },
    CounterRename {
        name: String,
        new_name: String,
    },
    CounterReset {
        name: String,
    },
    UuidNew {
        name: String,
    },
    UuidRead {
        name: String,
    },
    UuidRm {
        name: String,
    },
    List {
        filter_type: Option<String>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Response {
    Ok {
        #[serde(skip_serializing_if = "Option::is_none")]
        value: Option<serde_json::Value>,
    },
    Error {
        message: String,
    },
}

impl Response {
    pub fn ok_empty() -> Self {
        Response::Ok { value: None }
    }

    pub fn ok_value(value: serde_json::Value) -> Self {
        Response::Ok { value: Some(value) }
    }

    pub fn error(message: impl Into<String>) -> Self {
        Response::Error {
            message: message.into(),
        }
    }
}
