/// Standardized AETHER compiler error code registry.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ErrorCode {
    // Lexer errors (E0001-E0099)
    InvalidCharacter,   // E0001
    UnterminatedString, // E0002
    InvalidNumber,      // E0003
    UnknownToken,       // E0004

    // Parser errors (E0100-E0199)
    UnexpectedToken, // E0100
    MissingToken,    // E0101
    InvalidSyntax,   // E0102

    // Type checker errors (E0200-E0299)
    TypeMismatch,  // E0200
    UndefinedType, // E0201
    OccursCheck,   // E0202

    // Module errors (E0300-E0399)
    ModuleNotFound,     // E0300
    CircularDependency, // E0301
    ImportNotFound,     // E0302
}

impl ErrorCode {
    /// Return the standardized error code tag.
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::InvalidCharacter => "E0001",
            ErrorCode::UnterminatedString => "E0002",
            ErrorCode::InvalidNumber => "E0003",
            ErrorCode::UnknownToken => "E0004",
            ErrorCode::UnexpectedToken => "E0100",
            ErrorCode::MissingToken => "E0101",
            ErrorCode::InvalidSyntax => "E0102",
            ErrorCode::TypeMismatch => "E0200",
            ErrorCode::UndefinedType => "E0201",
            ErrorCode::OccursCheck => "E0202",
            ErrorCode::ModuleNotFound => "E0300",
            ErrorCode::CircularDependency => "E0301",
            ErrorCode::ImportNotFound => "E0302",
        }
    }
}
