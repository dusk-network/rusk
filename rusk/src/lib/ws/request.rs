use serde_json::{Map, Value};

use crate::error::{Error, WsRequestError};

type Headers = Map<String, Value>;

/// A request sent by the websocket client.
pub(crate) struct Request {
    headers: Headers,
    target_type: u8,
    target_name: String,
    event_topic: String,
    data: Vec<u8>,
}

impl Request {
    pub fn parse(bytes: &[u8]) -> Result<Self, Error> {
        let header_len_bytes = bytes.get(0..4);

        let header_len = parse_header_len(header_len_bytes)?;
        let headers_bytes = bytes.get(4..(4 + header_len as usize));

        let headers = parse_headers(headers_bytes, header_len)?;
        let event_bytes = bytes.get((4 + header_len as usize)..);

        let (target_type, target_name, event_topic, data) =
            parse_event(event_bytes)?;

        Ok(Self {
            headers,
            target_type,
            target_name: target_name.to_owned(),
            event_topic: event_topic.to_owned(),
            data: data.to_vec(),
        })
    }
}

fn parse_header_len(bytes: Option<&[u8]>) -> Result<u32, Error> {
    match bytes {
        Some(header_len) => {
            if header_len.len() < 4 {
                return Err(Error::WebSocketRequest(
                    WsRequestError::HeaderLength,
                ));
            }
            /// PANIC: we check if length is at least 4 above
            let len = u32::from_le_bytes([
                header_len[0],
                header_len[1],
                header_len[2],
                header_len[3],
            ]);

            Ok(len)
        }
        None => Err(Error::WebSocketRequest(WsRequestError::HeaderLength)),
    }
}

fn parse_headers(bytes: Option<&[u8]>, len: u32) -> Result<Headers, Error> {
    match bytes {
        Some(headers_bytes) => {
            if headers_bytes.len() != (len as usize) {
                return Err(Error::WebSocketRequest(
                    WsRequestError::HeaderLength,
                ));
            };

            let utf_bytes =
                std::str::from_utf8(headers_bytes).map_err(|e| {
                    Error::WebSocketRequest(
                        WsRequestError::HeadersSerialization(
                            serde_json::error::Category::Data,
                        ),
                    )
                })?;

            if let Value::Object(headers) =
                serde_json::from_str::<Value>(utf_bytes).map_err(|e| {
                    Error::WebSocketRequest(
                        WsRequestError::HeadersSerialization(e.classify()),
                    )
                })?
            {
                Ok(headers)
            } else {
                Err(Error::WebSocketRequest(
                    WsRequestError::HeadersSerialization(
                        serde_json::error::Category::Data,
                    ),
                ))
            }
        }
        None => Err(Error::WebSocketRequest(WsRequestError::HeaderLength)),
    }
}

fn parse_event(bytes: Option<&[u8]>) -> Result<(u8, &str, &str, &[u8]), Error> {
    match bytes {
        Some(bytes) => {
            // check if we have first 5 bytes to get to t_name length
            if bytes.len() < 5 {
                return Err(Error::WebSocketRequest(WsRequestError::Event));
            }

            let t_type = u8::from_le_bytes([bytes[0]]);

            // we have 3 target types only
            if t_type > 3 {
                return Err(Error::WebSocketRequest(
                    WsRequestError::UnknownTargetType,
                ));
            }

            let t_name_length =
                u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]);

            let offset_bytes = t_name_length as usize + 5;

            let t_name = bytes
                .get(5..(offset_bytes))
                .ok_or(Error::WebSocketRequest(WsRequestError::TargetName))?;

            let t_name_string = std::str::from_utf8(t_name).map_err(|_| {
                Error::WebSocketRequest(WsRequestError::TargetName)
            })?;

            let topic_length_bytes = bytes
                .get(offset_bytes..(offset_bytes + 4))
                .ok_or(Error::WebSocketRequest(WsRequestError::TopicName))?;

            let event_topic_length = u32::from_le_bytes([
                topic_length_bytes[0],
                topic_length_bytes[1],
                topic_length_bytes[2],
                topic_length_bytes[3],
            ]);

            let event_topic = bytes
                .get(
                    (offset_bytes + 4)
                        ..(offset_bytes + 4 + event_topic_length as usize),
                )
                .ok_or(Error::WebSocketRequest(WsRequestError::TopicName))?;

            let topic_string =
                std::str::from_utf8(event_topic).map_err(|_| {
                    Error::WebSocketRequest(WsRequestError::TopicName)
                })?;

            let data = bytes
                .get(offset_bytes + 4 + event_topic_length as usize..)
                .ok_or(Error::WebSocketRequest(WsRequestError::Data))?;

            Ok((t_type, t_name_string, topic_string, data))
        }
        None => Err(Error::WebSocketRequest(WsRequestError::UnknownTargetType)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use serde_json::json;

    #[test]
    fn basic_request_parsing() {
        let headers = r#"
        {
            "Content-Type" : "application/json",
            "Rusk-Version" : "0.10.2",
            "X-Request-Id" : "12be34ef",
            "Content-Length": 250
        }"#;

        let event_target_type = 1u8;
        let target_name = "event!";
        let target_name_length = target_name.len() as u32;
        let event_topic = "TOPIC!";
        let event_topic_length = event_topic.len() as u32;
        let data = "This is the data";

        let mut event = Vec::new();

        event.extend(event_target_type.to_le_bytes());
        event.extend(target_name_length.to_le_bytes());
        event.extend(target_name.as_bytes());
        event.extend(event_topic_length.to_le_bytes());
        event.extend(event_topic.as_bytes());
        event.extend(data.as_bytes());

        let mut message = Vec::new();

        message.extend((headers.len() as u32).to_le_bytes());
        message.extend(headers.as_bytes());
        message.extend(event.clone());

        let req = Request::parse(&message).expect("failed");

        assert_eq!(req.target_type, 1);
        assert_eq!(req.target_name, "event!");
        assert_eq!(req.event_topic, "TOPIC!");
    }

    #[test]
    #[should_panic]
    fn invalid_target_type() {
        let headers = r#"
        {
            "Content-Type" : "application/json",
            "Rusk-Version" : "0.10.2",
            "X-Request-Id" : "12be34ef",
            "Content-Length": 250
        }"#;

        let event_target_type = 99u8;
        let target_name = "event!";
        let target_name_length = target_name.len() as u32;
        let event_topic = "TOPIC!";
        let event_topic_length = event_topic.len() as u32;
        let data = "This is the data";

        let mut event = Vec::new();

        event.extend(event_target_type.to_le_bytes());
        event.extend(target_name_length.to_le_bytes());
        event.extend(target_name.as_bytes());
        event.extend(event_topic_length.to_le_bytes());
        event.extend(event_topic.as_bytes());
        event.extend(data.as_bytes());

        let mut message = Vec::new();

        message.extend((headers.len() as u32).to_le_bytes());
        message.extend(headers.as_bytes());
        message.extend(event.clone());

        Request::parse(&message).expect("panic");
    }

    #[test]
    #[should_panic]
    fn invalid_lenghts() {
        let headers = r#"
        {
            "Content-Type" : "application/json",
            "Rusk-Version" : "0.10.2",
            "X-Request-Id" : "12be34ef",
            "Content-Length": 250
        }"#;

        let event_target_type = 1u8;
        let target_name = "event!";
        let target_name_length = target_name.len() as u32 + 2;
        let event_topic = "TOPIC!";
        let event_topic_length = event_topic.len() as u32 + 2;
        let data = "This is the data";

        let mut event = Vec::new();

        event.extend(event_target_type.to_le_bytes());
        event.extend(target_name_length.to_le_bytes());
        event.extend(target_name.as_bytes());
        event.extend(event_topic_length.to_le_bytes());
        event.extend(event_topic.as_bytes());
        event.extend(data.as_bytes());

        let mut message = Vec::new();

        message.extend((headers.len() as u32).to_le_bytes());
        message.extend(headers.as_bytes());
        message.extend(event.clone());

        Request::parse(&message).expect("failed");
    }
}
