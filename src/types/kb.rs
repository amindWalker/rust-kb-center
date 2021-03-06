use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KnowledgeBase {
    pub id: KBId,
    pub title: String,
    pub content: String,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub struct KBId(pub i32);

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct NewKB {
    pub title: String,
    pub content: String,
    pub tags: Option<Vec<String>>,
}
