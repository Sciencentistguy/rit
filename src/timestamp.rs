use crate::Result;
use std::fmt::Display;

use chrono::{DateTime, FixedOffset};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp(pub DateTime<FixedOffset>);

impl Timestamp {
    pub fn now() -> Self {
        Self(chrono::offset::Local::now().into())
    }
    /// Parse a timestamp from a unix + HH + mm offset
    pub fn from_git(s: &str) -> Result<Self> {
        Ok(Self(DateTime::parse_from_str(s, "%s %z")?))
    }
}

impl Display for Timestamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0.format("%s %z"))
    }
}
