//! Primitive serializable and deserializable types.

use serde::de;
use serde_repr::{Deserialize_repr, Serialize_repr};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

/// The source of a line of output.
#[derive(Clone, Serialize_repr, Deserialize_repr, sqlx::Type)]
#[repr(u8)]
pub enum Source {
    Internal = 0,
    Stdout = 1,
    Stderr = 2,
}

/// The direction a measured value improves in.
#[derive(Clone, Serialize_repr, Deserialize_repr, sqlx::Type)]
#[repr(i8)]
pub enum Direction {
    LessIsBetter = -1,
    Neutral = 0,
    MoreIsBetter = 1,
}

/// How a commit can be reached from refs.
#[derive(Debug, Clone, Serialize_repr, Deserialize_repr, sqlx::Type)]
#[repr(u8)]
pub enum Reachable {
    Unreachable = 0,
    FromAnyRef = 1,
    FromTrackedRef = 2,
}

/// A time stamp, usually formatted using RFC3339.
#[derive(Clone, Copy, sqlx::Type)]
#[sqlx(transparent)]
pub struct Timestamp(pub OffsetDateTime);

impl serde::Serialize for Timestamp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.format(&Rfc3339).unwrap().serialize(serializer)
    }
}

impl<'de> serde::Deserialize<'de> for Timestamp {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let input: &str = serde::Deserialize::deserialize(deserializer)?;
        OffsetDateTime::parse(input, &Rfc3339)
            .map_err(de::Error::custom)
            .map(Self)
    }
}
