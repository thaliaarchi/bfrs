use std::{
    mem,
    ops::{Add, AddAssign},
};

use crate::egraph::{Graph, NodeId};

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

/// An ID for a basic block, unique per e-graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

/// An ID for an input, unique per e-graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputId(pub u32);

/// A relative offset to the cell pointer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Offset(pub i64);

impl Node {
    /// Inserts this node into the e-graph and transforms it to its ideal
    /// representation. Any structurally equivalent nodes are deduplicated and
    /// receive the same ID.
    pub fn insert(self, g: &mut Graph) -> NodeId {
        match self {
            Node::Add(mut lhs, mut rhs) => {
                if let Node::Add(b, c) = g[rhs] {
                    if let Node::Add(..) = g[lhs] {
                        return Node::Add(Node::Add(lhs, b).insert(g), c).insert(g);
                    }
                    mem::swap(&mut lhs, &mut rhs);
                }
                let (tail, head) = match g[lhs] {
                    Node::Add(a, b) => (Some(a), b),
                    _ => (None, lhs),
                };
                let (res, idealize) = match (&g[head], &g[rhs]) {
                    (&Node::Const(a), &Node::Const(b)) => {
                        (Node::Const(a.wrapping_add(b)).insert_ideal(g), true)
                    }
                    (_, Node::Const(0)) => (head, false),
                    (Node::Const(0), _) => (rhs, true),
                    (Node::Const(_), _) => {
                        if let Some(tail) = tail {
                            return Node::Add(Node::Add(tail, rhs).insert(g), head).insert(g);
                        } else {
                            return Node::Add(rhs, head).insert_ideal(g);
                        }
                    }
                    _ if head == rhs => (
                        Node::Mul(head, Node::Const(2).insert_ideal(g)).insert(g),
                        true,
                    ),
                    (&Node::Mul(a, b), _) if a == rhs => {
                        let n = Node::Add(b, Node::Const(1).insert_ideal(g)).insert(g);
                        (Node::Mul(a, n).insert(g), true)
                    }
                    (_, &Node::Mul(b, c)) if b == head => {
                        let n = Node::Add(c, Node::Const(1).insert_ideal(g)).insert(g);
                        (Node::Mul(b, n).insert(g), true)
                    }
                    _ => return Node::Add(lhs, rhs).insert_ideal(g),
                };
                if let Some(tail) = tail {
                    if res == head {
                        lhs
                    } else if idealize {
                        Node::Add(tail, res).insert(g)
                    } else {
                        Node::Add(tail, res).insert_ideal(g)
                    }
                } else {
                    res
                }
            }
            Node::Mul(mut lhs, mut rhs) => {
                if let Node::Mul(b, c) = g[rhs] {
                    if let Node::Mul(..) = g[lhs] {
                        return Node::Mul(Node::Mul(lhs, b).insert(g), c).insert(g);
                    }
                    mem::swap(&mut lhs, &mut rhs);
                }
                let (tail, head) = match g[lhs] {
                    Node::Mul(a, b) => (Some(a), b),
                    _ => (None, lhs),
                };
                let (res, idealize) = match (&g[head], &g[rhs]) {
                    (&Node::Const(a), &Node::Const(b)) => {
                        (Node::Const(a.wrapping_mul(b)).insert_ideal(g), true)
                    }
                    (_, Node::Const(1)) => (head, false),
                    (Node::Const(1), _) => (rhs, true),
                    (_, Node::Const(0)) | (Node::Const(0), _) => {
                        return Node::Const(0).insert_ideal(g)
                    }
                    (Node::Const(_), _) => {
                        if let Some(tail) = tail {
                            return Node::Mul(Node::Mul(tail, rhs).insert(g), head).insert(g);
                        } else {
                            return Node::Mul(rhs, head).insert_ideal(g);
                        }
                    }
                    _ => return Node::Mul(lhs, rhs).insert_ideal(g),
                };
                if let Some(tail) = tail {
                    if res == head {
                        lhs
                    } else if idealize {
                        Node::Mul(tail, res).insert(g)
                    } else {
                        Node::Mul(tail, res).insert_ideal(g)
                    }
                } else {
                    res
                }
            }
            _ => self.insert_ideal(g),
        }
    }

    /// Inserts this node into the e-graph, without idealizing. The node must
    /// already be idealized. Any structurally equivalent nodes are deduplicated
    /// and receive the same ID.
    pub fn insert_ideal(self, g: &mut Graph) -> NodeId {
        g.insert(self)
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
