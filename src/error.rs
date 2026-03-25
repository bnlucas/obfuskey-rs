#[derive(Debug, thiserror::Error)]
pub enum ObfuskeyError {
    #[error("The alphabet contains duplicate characters.")]
    DuplicateError,

    #[error("The multiplier must be an odd integer.")]
    MultiplierError,

    #[error("{0}")]
    NegativeValueError(String),

    #[error("{0}")]
    MaximumValueError(String),

    #[error("The key contains characters not found in the current alphabet.")]
    UnknownKeyError,

    #[error("The key length does not match the set length.")]
    KeyLengthError,

    #[error("{0}")]
    SchemaValidationError(String),

    #[error("{0}")]
    BitOverflowError(String),

    #[error("{0}")]
    ValueError(String),
}
