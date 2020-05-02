use anyhow::{anyhow, Error};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{convert::TryFrom, str::FromStr};
use tungstenite::Message;

/// Communication frame
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// sequence code
    cseq: u32,

    /// type of payload
    #[serde(flatten)]
    data: FrameData,
}

/// Type of payload
#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum FrameData {
    /// Subscribe request
    ///
    /// contains list of channels that client wants subscribe to
    Subscribe { channels: Vec<String> },

    /// Unsubscribe request
    ///
    /// contains list of channels that client wants to unsubscribe from
    Unsubscribe { channels: Vec<String> },

    /// Ready acknowledgement
    ///
    /// client signals that is ready to data transfer
    Ready,

    /// Ok Frame
    ///
    /// a status frame - server sucessfully processed the request
    Ok,

    /// Err frame
    ///
    /// a status frame - server failed to processed the request. Reuses http codes
    Err { code: u32, reason: String },

    /// Data message
    ///
    /// data sent by server to client
    Data { compressed: bool, payload: String },
}

impl Frame {
    /// Returns cseq of message
    pub fn cseq(&self) -> u32 {
        self.cseq
    }

    /// Returns frame type of message
    pub fn data(&self) -> &FrameData {
        &self.data
    }

    /// Creates "ok" frame basing on request
    ///
    /// # Arguments:
    /// * `client_frame` - request frame
    pub fn create_ok_frame(client_frame: &Frame) -> Frame {
        let cseq = client_frame.cseq;

        Frame {
            cseq,
            data: FrameData::Ok,
        }
    }

    /// Creates "err" frame basing on request
    ///
    /// # Arguments:
    /// * `client_frame` - request frame
    /// * `code` - code from HTTP range
    /// * `reason` - reason of error
    pub fn create_err_frame<S: Into<String>>(client_frame: &Frame, code: u32, reason: S) -> Frame {
        let cseq = client_frame.cseq;
        let reason = reason.into();

        Frame {
            cseq,
            data: FrameData::Err { code, reason },
        }
    }

    /// Creates data frame - a response for client frame
    ///
    /// # Arguments:
    /// * `client_frame` - request frame
    /// * `data` - payload to be sent
    pub fn create_data_frame(client_frame: &Frame, data: Value) -> Frame {
        let cseq = client_frame.cseq;
        let mut data = data.to_string();

        let compressed = data.len() > 100;

        if compressed {
            data = lz_string::compress_uri(&data).unwrap();
        }

        Frame {
            cseq,
            data: FrameData::Data {
                compressed,
                payload: data,
            },
        }
    }

    /// Converts `Frame` to websocket `Message`
    pub fn socket_msg(&self) -> Message {
        let serialized_text = serde_json::to_string(&self).expect("No reason to fail");

        Message::Text(serialized_text)
    }
}

impl FromStr for Frame {
    type Err = Error;

    fn from_str(message: &str) -> Result<Self, Self::Err> {
        let frame: Frame =
            serde_json::from_str(&message).map_err(|e| anyhow!("Deserialize error!\n\t{}", e))?;

        Ok(frame)
    }
}

impl TryFrom<&tungstenite::Message> for Frame {
    type Error = Error;

    fn try_from(message: &Message) -> Result<Self, Self::Error> {
        let message = match message {
            Message::Text(txt) => txt.trim(),
            _ => return Err(anyhow!("Expected Message::Text")),
        };

        let frame = message.parse()?;

        Ok(frame)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use serde_json::json;

    #[test]
    fn subscribe_deserialize() {
        let json = r#"{"cseq":1,"type":"subscribe","channels":["news"]}"#;

        let expected_msg = Frame {
            cseq: 1,
            data: FrameData::Subscribe {
                channels: vec!["news".to_string()],
            },
        };

        let msg = json.parse::<Frame>().unwrap();

        assert_eq!(msg, expected_msg);
    }

    #[test]
    fn data_frame() {
        let data = json!({"t": "xyz"});
        let ready_req = Frame {
            cseq: 2,
            data: FrameData::Ready,
        };

        let expected_frame = Frame {
            cseq: 2,
            data: FrameData::Data {
                compressed: false,
                payload: r#"{"t":"xyz"}"#.to_string(),
            },
        };

        let response_frame = Frame::create_data_frame(&ready_req, data);

        println!("response_frame {:?}", response_frame);
        assert_eq!(response_frame, expected_frame);
    }

    #[test]
    fn ready() {
        let frame = Frame {
            cseq: 1,
            data: FrameData::Ready,
        };

        let json = serde_json::to_string(&frame).unwrap();

        println!("json: {}", json);
    }
}
