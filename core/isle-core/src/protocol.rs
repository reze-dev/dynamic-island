use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum ClientMessage {
    #[serde(rename = "cmd")]
    Cmd {
        id: u64,
        name: String,
        #[serde(default)]
        args: serde_json::Value,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "t")]
pub enum ServerMessage {
    #[serde(rename = "state")]
    State { data: crate::state::Snapshot },

    #[serde(rename = "event")]
    Event {
        topic: String,
        data: serde_json::Value,
    },

    #[serde(rename = "reply")]
    Reply {
        id: u64,
        ok: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        error: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        data: Option<serde_json::Value>,
    },
}

impl ServerMessage {
    pub fn reply_ok(id: u64) -> Self {
        Self::Reply {
            id,
            ok: true,
            error: None,
            data: None,
        }
    }

    pub fn reply_with(id: u64, data: impl Into<serde_json::Value>) -> Self {
        Self::Reply {
            id,
            ok: true,
            error: None,
            data: Some(data.into()),
        }
    }

    pub fn reply_err(id: u64, error: impl Into<String>) -> Self {
        Self::Reply {
            id,
            ok: false,
            error: Some(error.into()),
            data: None,
        }
    }

    pub fn event(topic: impl Into<String>, data: serde_json::Value) -> Self {
        Self::Event {
            topic: topic.into(),
            data,
        }
    }

    pub fn encode(&self) -> String {
        let mut line = serde_json::to_string(self).expect("ServerMessage is always serializable");
        line.push('\n');
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_encodes_with_a_type_tag() {
        let snapshot = crate::state::Snapshot::default();
        let encoded = ServerMessage::State { data: snapshot }.encode();
        assert!(encoded.starts_with(r#"{"t":"state","data":"#), "{encoded}");
        assert!(encoded.ends_with('\n'));
    }

    #[test]
    fn reply_omits_error_on_success() {
        assert_eq!(
            ServerMessage::reply_ok(7).encode(),
            "{\"t\":\"reply\",\"id\":7,\"ok\":true}\n"
        );
    }

    #[test]
    fn reply_carries_error_on_failure() {
        assert_eq!(
            ServerMessage::reply_err(7, "nope").encode(),
            "{\"t\":\"reply\",\"id\":7,\"ok\":false,\"error\":\"nope\"}\n"
        );
    }

    #[test]
    fn cmd_round_trips_with_and_without_args() {
        let bare = r#"{"t":"cmd","id":1,"name":"view.show"}"#;
        assert_eq!(
            serde_json::from_str::<ClientMessage>(bare).unwrap(),
            ClientMessage::Cmd {
                id: 1,
                name: "view.show".into(),
                args: serde_json::Value::Null,
            }
        );

        let with_args = r#"{"t":"cmd","id":2,"name":"volume.set","args":{"level":0.4}}"#;
        assert_eq!(
            serde_json::from_str::<ClientMessage>(with_args).unwrap(),
            ClientMessage::Cmd {
                id: 2,
                name: "volume.set".into(),
                args: serde_json::json!({ "level": 0.4 }),
            }
        );
    }

    #[test]
    fn unknown_message_shapes_are_rejected() {
        assert!(serde_json::from_str::<ClientMessage>(r#"{"t":"nonsense"}"#).is_err());
        assert!(serde_json::from_str::<ServerMessage>(r#"{"t":"nonsense"}"#).is_err());
    }
}
