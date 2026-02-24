/// All errors that can arise during parsing, normalization, and lowering of a
/// Conclave source file.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum LangError {
    #[error("unexpected token at line {line}: expected {expected}, got '{got}'")]
    UnexpectedToken {
        expected: String,
        got: String,
        line: usize,
    },

    #[error("unexpected end of file: expected {expected}")]
    UnexpectedEof { expected: String },

    #[error("duplicate declaration: '{0}'")]
    DuplicateDeclaration(String),

    #[error("unknown capability or intrinsic: '{0}'")]
    UnknownCapability(String),

    #[error("version mismatch: expected '{expected}', got '{got}'")]
    VersionMismatch { expected: String, got: String },

    #[error("shadowed binding: '{0}' is already defined in this scope")]
    ShadowedBinding(String),

    #[error("undefined binding: '{0}'")]
    UndefinedBinding(String),

    #[error("no goals declared in module")]
    NoGoals,

    #[error("lowering requires url_count > 0 for map constructs")]
    MapRequiresUrlCount,

    #[error("invalid import hash for '{name}': '{hash}' (must be sha256:<64 hex chars>)")]
    InvalidImportHash { name: String, hash: String },

    #[error("pure block contains capability '{0}': only intrinsics are allowed")]
    PureBlockContainsCapability(String),

    #[error("reduce body must end with '{0} = <expr>;' accumulator assignment")]
    ReduceBodyMissingAssign(String),

    #[error("goal '{0}' not found in module")]
    GoalNotFound(String),

    #[error("import '{0}' cannot be resolved: no module cache provided")]
    ImportResolutionRequired(String),

    #[error("imported module not found in cache: '{0}'")]
    ImportNotFound(String),
}
