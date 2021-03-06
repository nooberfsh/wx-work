use xmltree::{Element, XMLNode};

use super::crypto::{Crypto, Payload};
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

    pub fn new_pic(media_id: String, to_user_name: String, from_user_name: String) -> SendMessage {
        let msg_ty = SendMessageType::Picture(media_id);
        SendMessage {
            to_user_name,
            from_user_name,
            msg_ty,
        }
    }

    pub fn new_voice(
        media_id: String,
        to_user_name: String,
        from_user_name: String,
    ) -> SendMessage {
        let msg_ty = SendMessageType::Voice(media_id);
        SendMessage {
            to_user_name,
            from_user_name,
            msg_ty,
        }
    }

    pub fn new_video(
        video: SendVideo,
        to_user_name: String,
        from_user_name: String,
    ) -> SendMessage {
        let msg_ty = SendMessageType::Video(video);
        SendMessage {
            to_user_name,
            from_user_name,
            msg_ty,
        }
    }

    pub fn new_pic_text(
        pt: PictureText,
        to_user_name: String,
        from_user_name: String,
    ) -> SendMessage {
        Self::new_pic_texts(vec![pt], to_user_name, from_user_name)
    }

    pub fn new_pic_texts(
        pts: Vec<PictureText>,
        to_user_name: String,
        from_user_name: String,
    ) -> SendMessage {
        let msg_ty = SendMessageType::PictureText(pts);
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

        let mut receiver = to_user_name.clone().into_bytes();

        let to = new_node("ToUserName", to_user_name);
        let from = new_node("FromUserName", from_user_name);
        let create_time = new_node("CreateTime", format!("{}", timestamp));
        let mut nodes = vec![to, from, create_time];

        match msg_ty {
            SendMessageType::Text(content) => {
                let msg_type = new_node("MsgType", "text".to_string());
                let content = new_node("Content", content);
                nodes.push(msg_type);
                nodes.push(content);
            }
            SendMessageType::Picture(media_id) => {
                let msg_type = new_node("MsgType", "image".to_string());
                let pic = new_node("MediaId", media_id);
                let pic_node = XMLNode::Element(new_xml("Image", vec![pic]));
                nodes.push(msg_type);
                nodes.push(pic_node);
                receiver.clear() // TODO: 遗失微信 bug
            }
            SendMessageType::Voice(media_id) => {
                let msg_type = new_node("MsgType", "voice".to_string());
                let voice = new_node("MediaId", media_id);
                let voice_node = XMLNode::Element(new_xml("Voice", vec![voice]));
                nodes.push(msg_type);
                nodes.push(voice_node);
            }
            SendMessageType::Video(v) => {
                let msg_type = new_node("MsgType", "video".to_string());
                let media_id = new_node("MediaId", v.media_id);
                let title = new_node("Title", v.title);
                let desc = new_node("Description", v.description);
                let video_node = XMLNode::Element(new_xml("Video", vec![media_id, title, desc]));
                nodes.push(msg_type);
                nodes.push(video_node);
            }
            SendMessageType::PictureText(pts) => {
                let msg_type = new_node("MsgType", "news".to_string());
                let count = new_node("ArticleCount", pts.len().to_string());

                let items = pts
                    .into_iter()
                    .map(|pt| {
                        let title = new_node("Title", pt.title);
                        let desc = new_node("Description", pt.description);
                        let pic_url = new_node("PicUrl", pt.pic_url);
                        let url = new_node("Url", pt.url);
                        XMLNode::Element(new_xml("item", vec![title, desc, pic_url, url]))
                    })
                    .collect();
                let articles = XMLNode::Element(new_xml("Articles", items));
                nodes.push(msg_type);
                nodes.push(count);
                nodes.push(articles);
                receiver.clear();
            }
        };
        let xml = new_xml("xml", nodes);
        let inner = serialize_xml(xml);
        let payload = Payload {
            data: inner.into_bytes(),
            receiver_id: receiver,
        };
        let encrypt = crypto.encrypt(&payload);
        let sign = crypto.sign(encrypt.clone(), timestamp, nonce);

        let encrypt = new_node("Encrypt", encrypt);
        let msg_sig = new_node("MsgSignature", sign);
        let timestamp = new_node("TimeStamp", format!("{}", timestamp));
        let nonce = new_node("Nonce", format!("{}", nonce));

        let xml = new_xml("xml", vec![encrypt, msg_sig, timestamp, nonce]);
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

fn new_xml(name: &str, nodes: Vec<XMLNode>) -> Element {
    Element {
        prefix: None,
        namespace: None,
        namespaces: None,
        name: name.to_string(),
        attributes: Default::default(),
        children: nodes,
    }
}

fn serialize_xml(e: Element) -> String {
    let mut ret = vec![];
    e.write(&mut ret).unwrap();
    String::from_utf8(ret).unwrap()
}
