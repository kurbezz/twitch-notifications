use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatType {
    Private,
    Group,
    Supergroup,
    Channel,
}

impl ChatType {
    /// Convert from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "private" => Some(ChatType::Private),
            "group" => Some(ChatType::Group),
            "supergroup" => Some(ChatType::Supergroup),
            "channel" => Some(ChatType::Channel),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(self) -> &'static str {
        match self {
            ChatType::Private => "private",
            ChatType::Group => "group",
            ChatType::Supergroup => "supergroup",
            ChatType::Channel => "channel",
        }
    }
}

impl From<ChatType> for String {
    fn from(chat_type: ChatType) -> Self {
        chat_type.as_str().to_string()
    }
}

impl TryFrom<String> for ChatType {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::from_str(&value).ok_or_else(|| format!("Invalid chat type: {}", value))
    }
}

impl TryFrom<&str> for ChatType {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::from_str(value).ok_or_else(|| format!("Invalid chat type: {}", value))
    }
}
