use itertools::Itertools;
use serde::ser::{SerializeMap, Serializer};
use serde::Serialize;
use thiserror::Error;

pub struct MessageResponse {
    pub errcode: u64,
    pub errmsg: String,
    pub invaliduser: String,
    pub invalidparty: String,
    pub invalidtag: String,
}

#[derive(Error, Debug)]
pub enum MessageBuildError {
    #[error("receiver can not be empty")]
    EmptyReceiver,
}

pub struct MessageBuilder {
    to_users: Vec<String>,
    to_parties: Vec<String>,
    to_tags: Vec<String>,
    data: MessageType,
    agent_id: u64,
    safe: Option<bool>,
    enable_id_trans: Option<bool>,
    enable_duplicate_check: Option<bool>,
    duplicate_check_interval: Option<u32>,
}

#[derive(Debug)]
pub struct Message {
    to_users: Vec<String>, //指定接收消息的成员，成员ID列表（多个接收者用‘|’分隔，最多支持1000个）。特殊情况：指定为”@all”，则向该企业应用的全部成员发送
    to_parties: Vec<String>, //指定接收消息的部门，部门ID列表，多个接收者用‘|’分隔，最多支持100个。当touser为”@all”时忽略本参数
    to_tags: Vec<String>, // 指定接收消息的标签，标签ID列表，多个接收者用‘|’分隔，最多支持100个。当touser为”@all”时忽略本参数
    data: MessageType,    // 数据
    agent_id: u64, // 企业应用的id，整型。企业内部开发，可在应用的设置页面查看；第三方服务商，可通过接口 获取企业授权信息 获取该参数值
    safe: Option<bool>, // 表示是否是保密消息，0表示否，1表示是，默认0
    enable_id_trans: Option<bool>, // 表示是否开启id转译，0表示否，1表示是，默认0
    enable_duplicate_check: Option<bool>, // 表示是否开启重复消息检查，0表示否，1表示是，默认0
    duplicate_check_interval: Option<u32>, // 表示是否重复消息检查的时间间隔，默认1800s，最大不超过4小时
}

#[derive(Debug)]
enum MessageType {
    Text(Text),
    File(File),
}

#[derive(Debug, Serialize)]
struct Text {
    content: String,
}

#[derive(Debug, Serialize)]
struct File {
    media_id: String,
}

impl MessageBuilder {
    fn new(agent_id: u64, ty: MessageType) -> Self {
        MessageBuilder {
            agent_id,
            to_users: vec![],
            to_parties: vec![],
            to_tags: vec![],
            data: ty,
            safe: None,
            enable_id_trans: None,
            enable_duplicate_check: None,
            duplicate_check_interval: None,
        }
    }
    pub fn new_text(agent_id: u64, content: String) -> Self {
        let data = MessageType::Text(Text { content });
        Self::new(agent_id, data)
    }

    pub fn new_file(agent_id: u64, media_id: String) -> Self {
        let data = MessageType::File(File { media_id });
        Self::new(agent_id, data)
    }

    pub fn with_user(mut self, user: String) -> Self {
        self.to_users.push(user);
        self
    }

    pub fn with_party(mut self, party: String) -> Self {
        self.to_parties.push(party);
        self
    }

    pub fn with_tag(mut self, tag: String) -> Self {
        self.to_tags.push(tag);
        self
    }

    pub fn safe(mut self, flag: bool) -> Self {
        self.safe = Some(flag);
        self
    }

    pub fn enable_id_trans(mut self, flag: bool) -> Self {
        self.enable_id_trans = Some(flag);
        self
    }

    pub fn enable_duplicate_check(mut self, flag: bool) -> Self {
        self.enable_duplicate_check = Some(flag);
        self
    }

    pub fn duplicate_check_interval(mut self, duration: u32) -> Self {
        self.duplicate_check_interval = Some(duration);
        self
    }

    pub fn build(self) -> Result<Message, MessageBuildError> {
        if self.to_users.is_empty() || self.to_parties.is_empty() || self.to_tags.is_empty() {
            return Err(MessageBuildError::EmptyReceiver);
        }

        // TODO: add more checks
        let ret = Message {
            to_users: self.to_users,
            to_parties: self.to_parties,
            to_tags: self.to_tags,
            data: self.data,
            agent_id: self.agent_id,
            safe: self.safe,
            enable_id_trans: self.enable_id_trans,
            enable_duplicate_check: self.enable_duplicate_check,
            duplicate_check_interval: self.duplicate_check_interval,
        };
        Ok(ret)
    }
}

impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use MessageType::*;

        let mut map = serializer.serialize_map(None)?;

        map.serialize_entry("touser", &self.to_users.iter().join("|"))?;
        map.serialize_entry("toparty", &self.to_parties.iter().join("|"))?;
        map.serialize_entry("totag", &self.to_tags.iter().join("|"))?;
        map.serialize_entry("agentid", &self.agent_id)?;

        if let Some(d) = self.safe {
            map.serialize_entry("safe", &(d as u8))?;
        }
        if let Some(d) = self.enable_id_trans {
            map.serialize_entry("enable_id_trans", &(d as u8))?;
        }
        if let Some(d) = self.enable_duplicate_check {
            map.serialize_entry("enable_duplicate_check", &(d as u8))?;
        }
        if let Some(d) = self.duplicate_check_interval {
            map.serialize_entry("duplicate_check_interval", &d)?;
        }

        match &self.data {
            Text(t) => {
                map.serialize_entry("msgtype", "text")?;
                map.serialize_entry("text", t)?;
            }
            File(t) => {
                map.serialize_entry("msgtype", "file")?;
                map.serialize_entry("file", t)?;
            }
        }

        map.end()
    }
}
