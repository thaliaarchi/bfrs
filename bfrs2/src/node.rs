use std::ops::{Add, AddAssign};

use crate::arena::{Arena, NodeId};

/// A node for a byte computation.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Node {
    /// Copy the byte from the cell at the offset.
    Copy(Offset, BlockId),
    /// A constant byte.
    Const(u8),
    /// A byte read from the user.
    Input(InputId),
    /// Addition of two bytes.
    Add(NodeId, NodeId),
}

/// An ID for a basic block, unique per arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

/// An ID for an input, unique per arena.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputId(pub u32);

/// A relative offset to the cell pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Offset(pub i64);

impl Node {
    /// Inserts this node into the arena and returns its unique ID. Any
    /// structurally equivalent nodes are deduplicated and receive the same ID.
    pub fn insert(self, a: &mut Arena) -> NodeId {
        a.insert(self)
    }
}

impl Add<i64> for Offset {
    type Output = Self;

    fn add(self, rhs: i64) -> Self::Output {
        Offset(self.0 + rhs)
    }
}

impl AddAssign<i64> for Offset {
    fn add_assign(&mut self, rhs: i64) {
        *self = *self + rhs;
    }
}
