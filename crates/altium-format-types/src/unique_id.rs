use std::fmt;
use std::str::FromStr;

/// Error returned when constructing a UniqueId from an invalid string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UniqueIdError {
    pub input: String,
    pub reason: &'static str,
}

impl fmt::Display for UniqueIdError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid UniqueId {:?}: {}", self.input, self.reason)
    }
}

impl std::error::Error for UniqueIdError {}

/// 8-character uppercase alphabetic identifier (e.g., "LVUUGVHQ").
/// Validated at construction; construction from arbitrary strings is fallible.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UniqueId(String);

impl UniqueId {
    fn validate(s: &str) -> Result<(), &'static str> {
        if s.len() != 8 {
            return Err("must be exactly 8 characters");
        }
        if !s.chars().all(|c| c.is_ascii_uppercase()) {
            return Err("must contain only uppercase ASCII letters [A-Z]");
        }
        Ok(())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl FromStr for UniqueId {
    type Err = UniqueIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::validate(s).map_err(|reason| UniqueIdError {
            input: s.to_owned(),
            reason,
        })?;
        Ok(Self(s.to_owned()))
    }
}

impl fmt::Display for UniqueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
