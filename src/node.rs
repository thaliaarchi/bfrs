use std::{
    mem,
    ops::{Add, AddAssign},
};

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
    /// Multiplication of two bytes.
    Mul(NodeId, NodeId),
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
    /// Inserts this node into the arena and transforms it to its ideal
    /// representation. Any structurally equivalent nodes are deduplicated and
    /// receive the same ID.
    pub fn insert(self, a: &mut Arena) -> NodeId {
        match self {
            Node::Add(mut lhs, mut rhs) => {
                if let Node::Add(b, c) = a[rhs] {
                    if let Node::Add(..) = a[lhs] {
                        return Node::Add(Node::Add(lhs, b).insert(a), c).insert(a);
                    }
                    mem::swap(&mut lhs, &mut rhs);
                }
                let (tail, head) = match a[lhs] {
                    Node::Add(a, b) => (Some(a), b),
                    _ => (None, lhs),
                };
                let (res, idealize) = match (&a[head], &a[rhs]) {
                    (&Node::Const(x), &Node::Const(y)) => {
                        (Node::Const(x.wrapping_add(y)).insert_ideal(a), true)
                    }
                    (_, Node::Const(0)) => (head, false),
                    (Node::Const(0), _) => (rhs, true),
                    (Node::Const(_), _) => {
                        if let Some(tail) = tail {
                            return Node::Add(Node::Add(tail, rhs).insert(a), head).insert(a);
                        } else {
                            return Node::Add(rhs, head).insert_ideal(a);
                        }
                    }
                    _ if head == rhs => (
                        Node::Mul(head, Node::Const(2).insert_ideal(a)).insert(a),
                        true,
                    ),
                    (&Node::Mul(x, y), _) if x == rhs => {
                        let n = Node::Add(y, Node::Const(1).insert_ideal(a)).insert(a);
                        (Node::Mul(x, n).insert(a), true)
                    }
                    (_, &Node::Mul(b, c)) if b == head => {
                        let n = Node::Add(c, Node::Const(1).insert_ideal(a)).insert(a);
                        (Node::Mul(b, n).insert(a), true)
                    }
                    _ => return Node::Add(lhs, rhs).insert_ideal(a),
                };
                if let Some(tail) = tail {
                    if res == head {
                        lhs
                    } else if idealize {
                        Node::Add(tail, res).insert(a)
                    } else {
                        Node::Add(tail, res).insert_ideal(a)
                    }
                } else {
                    res
                }
            }
            Node::Mul(mut lhs, mut rhs) => {
                if let Node::Mul(b, c) = a[rhs] {
                    if let Node::Mul(..) = a[lhs] {
                        return Node::Mul(Node::Mul(lhs, b).insert(a), c).insert(a);
                    }
                    mem::swap(&mut lhs, &mut rhs);
                }
                let (tail, head) = match a[lhs] {
                    Node::Mul(a, b) => (Some(a), b),
                    _ => (None, lhs),
                };
                let (res, idealize) = match (&a[head], &a[rhs]) {
                    (&Node::Const(x), &Node::Const(y)) => {
                        (Node::Const(x.wrapping_mul(y)).insert_ideal(a), true)
                    }
                    (_, Node::Const(1)) => (head, false),
                    (Node::Const(1), _) => (rhs, true),
                    (_, Node::Const(0)) | (Node::Const(0), _) => {
                        return Node::Const(0).insert_ideal(a)
                    }
                    (Node::Const(_), _) => {
                        if let Some(tail) = tail {
                            return Node::Mul(Node::Mul(tail, rhs).insert(a), head).insert(a);
                        } else {
                            return Node::Mul(rhs, head).insert_ideal(a);
                        }
                    }
                    _ => return Node::Mul(lhs, rhs).insert_ideal(a),
                };
                if let Some(tail) = tail {
                    if res == head {
                        lhs
                    } else if idealize {
                        Node::Mul(tail, res).insert(a)
                    } else {
                        Node::Mul(tail, res).insert_ideal(a)
                    }
                } else {
                    res
                }
            }
            _ => self.insert_ideal(a),
        }
    }

    /// Inserts this node into the arena, without idealizing. The node must
    /// already be idealized. Any structurally equivalent nodes are deduplicated
    /// and receive the same ID.
    pub fn insert_ideal(self, a: &mut Arena) -> NodeId {
        a.insert_ideal(self)
    }
}

impl Offset {
    /// Subtracts this offset from the minimum offset to get an index. Panics
    /// when `min > self`.
    pub fn index_from(self, min: Offset) -> usize {
        (self.0 - min.0)
            .try_into()
            .ok()
            .expect("BUG: offset before minimum")
    }

    /// Subtracts this offset from the minimum offset to get an index.
    pub fn try_index_from(self, min: Offset) -> Option<usize> {
        (self.0 - min.0).try_into().ok()
    }

    /// Subtracts this offset from the minimum offset to get a signed index.
    pub fn index_from_signed(self, min: Offset) -> isize {
        (self.0 - min.0).try_into().unwrap()
    }
}

impl Add for Offset {
    type Output = Self;

    fn add(self, rhs: Offset) -> Self::Output {
        Offset(self.0 + rhs.0)
    }
}

impl AddAssign for Offset {
    fn add_assign(&mut self, rhs: Offset) {
        *self = *self + rhs;
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
