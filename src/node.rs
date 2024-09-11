use std::mem;

use crate::{
    graph::{Graph, NodeId, NodeRef},
    ir::BasicBlock,
};

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
        match self {
            Node::Copy(_) | Node::Const(_) | Node::Input { .. } => self.insert(g),
            Node::Add(mut lhs, mut rhs) => {
                if let Node::Const(a) = g[lhs] {
                    if let Node::Const(b) = g[rhs] {
                        return Node::Const(a.wrapping_add(b)).insert(g);
                    }
                    mem::swap(&mut lhs, &mut rhs);
                }
                match (&g[lhs], &g[rhs]) {
                    (_, Node::Const(0)) => lhs,
                    (_, _) if lhs == rhs => Node::Mul(lhs, Node::Const(2).insert(g)).idealize(g),
                    (_, &Node::Add(b, c)) => {
                        Node::Add(Node::Add(lhs, b).idealize(g), c).idealize(g)
                    }
                    (&Node::Add(a, b), _) => match (&g[b], &g[rhs]) {
                        (&Node::Const(b), &Node::Const(c)) => {
                            Node::Add(a, Node::Const(b.wrapping_add(c)).insert(g)).idealize(g)
                        }
                        (&Node::Const(_), _) => {
                            Node::Add(Node::Add(a, rhs).idealize(g), b).idealize(g)
                        }
                        _ => Node::Add(lhs, rhs).insert(g),
                    },
                    _ => Node::Add(lhs, rhs).insert(g),
                }
            }
            Node::Mul(mut lhs, mut rhs) => {
                if let Node::Const(a) = g[lhs] {
                    if let Node::Const(b) = g[rhs] {
                        return Node::Const(a.wrapping_mul(b)).insert(g);
                    }
                    mem::swap(&mut lhs, &mut rhs);
                }
                match (&g[lhs], &g[rhs]) {
                    (_, Node::Const(1)) => lhs,
                    (_, Node::Const(0)) => Node::Const(0).insert(g),
                    (_, &Node::Mul(b, c)) => {
                        Node::Mul(Node::Mul(lhs, b).idealize(g), c).idealize(g)
                    }
                    (&Node::Mul(a, b), _) => match (&g[b], &g[rhs]) {
                        (&Node::Const(b), &Node::Const(c)) => {
                            Node::Mul(a, Node::Const(b.wrapping_mul(c)).insert(g)).idealize(g)
                        }
                        (&Node::Const(_), _) => {
                            Node::Mul(Node::Mul(a, rhs).idealize(g), b).idealize(g)
                        }
                        _ => Node::Mul(lhs, rhs).insert(g),
                    },
                    _ => Node::Mul(lhs, rhs).insert(g),
                }
            }
        }
    }

    /// Gets or inserts this node into a graph and returns its ID.
    #[inline]
    pub fn insert(self, g: &mut Graph) -> NodeId {
        g.insert(self)
    }

    /// Gets the ID of this node in a graph.
    #[inline]
    pub fn find(&self, g: &Graph) -> Option<NodeId> {
        g.find(self)
    }
}

impl NodeRef<'_> {
    /// Returns whether this node references a cell besides at the given offset.
    pub fn references_other(&self, offset: isize) -> bool {
        match *self.node() {
            Node::Copy(offset2) => offset2 != offset,
            Node::Const(_) | Node::Input { .. } => false,
            Node::Add(lhs, rhs) | Node::Mul(lhs, rhs) => {
                lhs.get(self.graph()).references_other(offset)
                    || rhs.get(self.graph()).references_other(offset)
            }
        }
    }
}

impl NodeId {
    pub fn rebase(&self, bb: &mut BasicBlock, g: &mut Graph) -> NodeId {
        match g[*self] {
            Node::Copy(offset) => bb.cell(bb.offset() + offset, g),
            Node::Const(c) => Node::Const(c).insert(g),
            Node::Input { id } => Node::Input {
                id: id + bb.inputs(),
            }
            .insert(g),
            Node::Add(lhs, rhs) => Node::Add(lhs.rebase(bb, g), rhs.rebase(bb, g)).idealize(g),
            Node::Mul(lhs, rhs) => Node::Mul(lhs.rebase(bb, g), rhs.rebase(bb, g)).idealize(g),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{graph::Graph, node::Node};

    #[test]
    fn idealize_add() {
        let g = &mut Graph::new();
        let x = Node::Copy(0).idealize(g);
        let y = Node::Copy(2).idealize(g);
        let add = Node::Add(
            Node::Add(x, Node::Const(1).idealize(g)).idealize(g),
            Node::Add(y, Node::Const(3).idealize(g)).idealize(g),
        )
        .idealize(g);
        let expected = Node::Add(Node::Add(x, y).insert(g), Node::Const(4).insert(g)).insert(g);
        assert_eq!(g.get(add), g.get(expected));
    }
}
