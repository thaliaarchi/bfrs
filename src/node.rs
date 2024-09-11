use std::mem;

use crate::graph::{Graph, NodeId};

/// A node in a graph.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Node {
    /// Copy the value from the cell at the offset.
    Copy(isize),
    /// A constant value.
    Const(u8),
    /// A value read from the user.
    Input { id: usize },
    /// Addition of two values.
    Add(NodeId, NodeId),
    /// Multiplication of two values.
    Mul(NodeId, NodeId),
}

impl Node {
    /// Inserts this node into the graph and transforms it to its ideal
    /// representation.
    pub fn idealize(self, g: &mut Graph) -> NodeId {
        let node = match self {
            Node::Copy(_) | Node::Const(_) | Node::Input { .. } => self,
            Node::Add(mut lhs, mut rhs) => {
                if let Node::Const(_) = &g[lhs] {
                    mem::swap(&mut lhs, &mut rhs);
                }
                match (&g[lhs], &g[rhs]) {
                    (&Node::Const(lhs), &Node::Const(rhs)) => Node::Const(lhs.wrapping_add(rhs)),
                    (_, Node::Const(0)) => return lhs,
                    (_, _) if lhs == rhs => Node::Mul(lhs, Node::Const(2).insert(g)),
                    (&Node::Add(a, b), _) => match (&g[b], &g[rhs]) {
                        (&Node::Const(b), &Node::Const(c)) => {
                            Node::Add(a, Node::Const(b.wrapping_add(c)).insert(g))
                        }
                        (&Node::Const(_), _) => Node::Add(Node::Add(a, rhs).insert(g), b),
                        _ => Node::Add(Node::Add(a, b).insert(g), rhs),
                    },
                    (_, &Node::Add(b, c)) => Node::Add(Node::Add(lhs, b).insert(g), c),
                    _ => Node::Add(lhs, rhs),
                }
            }
            Node::Mul(mut lhs, mut rhs) => {
                if let Node::Const(_) = &g[lhs] {
                    mem::swap(&mut lhs, &mut rhs);
                }
                match (&g[lhs], &g[rhs]) {
                    (&Node::Const(lhs), &Node::Const(rhs)) => Node::Const(lhs.wrapping_mul(rhs)),
                    (_, Node::Const(1)) => return lhs,
                    (_, Node::Const(0)) => Node::Const(0),
                    (&Node::Mul(a, b), _) => match (&g[b], &g[rhs]) {
                        (&Node::Const(b), &Node::Const(c)) => {
                            Node::Mul(a, Node::Const(b.wrapping_mul(c)).insert(g))
                        }
                        (&Node::Const(_), _) => Node::Mul(Node::Mul(a, rhs).insert(g), b),
                        _ => Node::Mul(Node::Mul(a, b).insert(g), rhs),
                    },
                    (_, &Node::Mul(b, c)) => Node::Mul(Node::Mul(lhs, b).insert(g), c),
                    _ => Node::Mul(lhs, rhs),
                }
            }
        };
        g.insert(node)
    }

    /// Gets or inserts this node into a graph and returns its ID.
    #[inline]
    pub fn insert(self, g: &mut Graph) -> NodeId {
        g.insert(self)
    }

    /// Gets the ID of this node in a graph.
    #[inline]
    pub fn get(&self, g: &Graph) -> Option<NodeId> {
        g.get(self)
    }
}
