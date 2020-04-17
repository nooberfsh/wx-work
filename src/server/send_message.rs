use xmltree::{Element, XMLNode};

use super::crypto::Crypto;
use super::error::MessageError::EncryptFailed;
use super::error::Result;

#[derive(Debug, Clone)]
pub struct SendMessage {
    pub to_user_name: String,
    pub from_user_name: String,
    pub msg_ty: SendMessageType,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum SendMessageType {
    Text(String),
    Picture(String), // media_id
    Voice(String),   // media_id
    Video(SendVideo),
    PictureText(Vec<PictureText>),
}

#[derive(Debug, Clone)]
pub struct SendVideo {
    pub media_id: String,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct PictureText {
    pub pic_url: String,
    pub url: String,
    pub title: String,
    pub description: String,
}

impl SendMessage {
    pub fn new_text(content: String, to_user_name: String, from_user_name: String) -> SendMessage {
        let msg_ty = SendMessageType::Text(content);
        SendMessage {
            to_user_name,
            from_user_name,
            msg_ty,
        }
    }

    pub(crate) fn serialize(self, timestamp: u64, nonce: u64, crypto: &Crypto) -> Result<String> {
        let SendMessage {
            to_user_name,
            from_user_name,
            msg_ty,
        } = self;
        let xml = match msg_ty {
            SendMessageType::Text(content) => {
                let to_user_name = new_node("ToUserName", to_user_name);
                let from_user_name = new_node("FromUserName", from_user_name);
                let create_time = new_node("CreateTime", format!("{}", timestamp));
                let msg_type = new_node("MsgType", "text".to_string());
                let content = new_node("Content", content);

                new_xml(vec![
                    to_user_name,
                    from_user_name,
                    create_time,
                    msg_type,
                    content,
                ])
            }
            _ => todo!(), // TODO
        };

        let inner = serialize_xml(xml);
        let encrypt = crypto
            .encrypt(inner.into_bytes())
            .map_err(|e| EncryptFailed(format!("{}", e)))?;
        let sign = crypto.sign(encrypt.clone(), timestamp, nonce);

        let encrypt = new_node("Encrypt", encrypt);
        let msg_sig = new_node("MsgSignature", sign);
        let timestamp = new_node("TimeStamp", format!("{}", timestamp));
        let nonce = new_node("Nonce", format!("{}", nonce));

        let xml = new_xml(vec![encrypt, msg_sig, timestamp, nonce]);
        let ret = serialize_xml(xml);
        Ok(ret)
    }
}

///////////////////////////// helper functions ///////////////////////////////////////////////

fn new_node(name: &str, data: String) -> XMLNode {
    let node = XMLNode::Text(data);
    let ret = Element {
        prefix: None,
        namespace: None,
        namespaces: None,
        name: name.to_string(),
        attributes: Default::default(),
        children: vec![node],
    };
    XMLNode::Element(ret)
}

fn new_xml(nodes: Vec<XMLNode>) -> Element {
    Element {
        prefix: None,
        namespace: None,
        namespaces: None,
        name: "xml".to_string(),
        attributes: Default::default(),
        children: nodes,
    }
}

fn serialize_xml(e: Element) -> String {
    let mut ret = vec![];
    e.write(&mut ret).unwrap();
    String::from_utf8(ret).unwrap()
}
