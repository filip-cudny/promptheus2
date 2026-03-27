use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "item_type", rename_all = "lowercase")]
pub enum ContextItem {
    Text { content: String },
    Image { data: String, media_type: String },
}
