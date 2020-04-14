use std::str::FromStr;

use xmltree::Element;

use crate::{crypto::Crypto, Error, Result};

#[derive(Debug, Clone)]
pub struct RecvMessage {
    pub msg_encrypt: String,
    pub to_user_name: String,
    pub from_user_name: String,
    pub agent_id: u64,
    pub create_time: u64,
    pub msg_id: u64,
    pub msg_ty: RecvMessageType,
}

// TODO: add event types
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum RecvMessageType {
    Text(String),
    Picture(Picture),
    Voice(Voice),
    Video(RecvVideo),
    Location(Location),
    Link(Link),
}

#[derive(Debug, Clone)]
pub struct Picture {
    pub pic_url: String,
    pub media_id: String,
}

#[derive(Debug, Clone)]
pub struct Voice {
    pub media_id: String,
    pub format: String,
}

#[derive(Debug, Clone)]
pub struct RecvVideo {
    pub media_id: String,
    pub thumb_media_id: String,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub location_x: f64,
    pub location_y: f64,
    pub scale: u32,
    pub label: String,
    pub ty: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Link {
    pub title: String,
    pub description: String,
    pub url: String,
    pub pic_url: String,
}

macro_rules! try_field {
    ($name:expr, $element:expr) => {
        match fetch($name, &$element) {
            Some(d) => d.to_string(),
            None => return Err(Error::MessageMissingField($name)),
        }
    };
}

macro_rules! try_field_parse {
    ($name:expr, $element:expr, $ty:ident) => {
        match fetch($name, &$element) {
            Some(d) => match $ty::from_str(d) {
                Ok(d) => d,
                Err(_) => {
                    return Err(Error::MessageInvalidFieldType(format!(
                        "{} parse failed",
                        $name
                    )))
                }
            },
            None => return Err(Error::MessageMissingField($name)),
        }
    };
}

fn fetch<'a>(name: &str, element: &'a Element) -> Option<&'a str> {
    let child = element.get_child(name)?;
    child.children.get(0)?.as_text()
}

impl RecvMessage {
    pub(crate) fn parse(
        data: impl AsRef<[u8]>,
        crypto: &Crypto,
        timestamp: u64,
        nonce: u64,
        msg_signature: &str,
    ) -> Result<RecvMessage> {
        let xml = Element::parse(data.as_ref())
            .map_err(|e| Error::MessageParseFailed(format!("{}", e)))?;

        let to_user_name = try_field!("ToUserName", xml);
        let agent_id = try_field_parse!("AgentID", xml, u64);
        let msg_encrypt = try_field!("Encrypt", xml);

        let sign = crypto.sign(msg_encrypt.clone(), timestamp, nonce);

        if sign != msg_signature {
            return Err(Error::InvalidMessage);
        }

        let msg = crypto.decrypt(&msg_encrypt)?.data;
        let inner_xml = Element::parse(&*msg)
            .map_err(|e| Error::MessageParseFailed(format!("inner: {}", e)))?;

        let from_user_name = try_field!("FromUserName", inner_xml);
        let create_time = try_field_parse!("CreateTime", inner_xml, u64);
        let msg_id = try_field_parse!("MsgId", inner_xml, u64);

        let msg_ty = match &*try_field!("MsgType", inner_xml) {
            "text" => {
                let content = try_field!("Content", inner_xml);
                RecvMessageType::Text(content)
            }
            ty => return Err(Error::MessageInvalidMessageType(ty.to_string())), // TODO
        };

        Ok(RecvMessage {
            msg_encrypt,
            to_user_name,
            agent_id,
            from_user_name,
            create_time,
            msg_id,
            msg_ty,
        })
    }
}
