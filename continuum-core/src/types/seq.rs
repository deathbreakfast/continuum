//! Monotonic sequence within a [`super::LogStreamId`].

use serde::{Deserialize, Serialize};

/// Monotonic sequence assigned by the backend on append within one stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Seq(
    /// Raw counter value; strictly increasing per [`super::LogStreamId`] on successful appends.
    pub i64,
);

impl Seq {
    /// Sequence value used before any events exist (`read_from` returns all when `after = ZERO`).
    pub const ZERO: Self = Self(0);

    /// Next sequence (saturating add).
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    /// Raw counter value.
    #[must_use]
    pub const fn as_i64(self) -> i64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering_and_next() {
        assert!(Seq(1) < Seq(2));
        assert_eq!(Seq(1).next(), Seq(2));
    }

    #[test]
    fn serde_roundtrip() {
        let s = Seq(42);
        let json = serde_json::to_string(&s).expect("serialize");
        let back: Seq = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(s, back);
    }
}
