/// Representation of import statements (use keyword).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImportKind {
    Wildcard,
    Selective(Vec<String>),
    Rename(String, String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportStatement {
    pub path: String,
    pub kind: ImportKind,
    pub is_public: bool,
}

impl ImportStatement {
    pub fn new(path: &str, kind: ImportKind, is_public: bool) -> Self {
        Self {
            path: path.to_string(),
            kind,
            is_public,
        }
    }
}
