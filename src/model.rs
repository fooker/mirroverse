use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Creator {
    pub id: u64,
    pub name: String,

    pub first_name: String,
    pub last_name: String,
}

#[derive(Debug, Serialize)]
pub struct Thing {
    pub id: u64,
    pub name: String,

    pub description: String,
    pub instructions: String,
    pub details: String,

    pub tags: Vec<String>,

    pub creator: Creator,
    pub license: String,
}
