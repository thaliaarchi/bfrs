use std::mem;

use crate::{
    graph::{ArrayId, ByteId, Graph, NodeId, NodeRef},
    ir::BasicBlock,
};

/// A node in a graph.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Node {
    Byte(Byte),
    Array(Array),
}

/// A byte value in a graph.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Byte {
    /// Copy the value from the cell at the offset.
    Copy(isize),
    /// A constant value.
    Const(u8),
    /// A value read from the user.
    Input { id: usize },
    /// Addition of two values.
    Add(ByteId, ByteId),
    /// Multiplication of two values.
    Mul(ByteId, ByteId),
}

/// An array with static size and dynamic elements.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Array {
    pub elements: Vec<ByteId>,
}

impl Byte {
    /// Inserts this byte node into the graph and transforms it to its ideal
    /// representation.
    pub fn idealize(self, g: &mut Graph) -> ByteId {
        match self {
            Byte::Copy(_) | Byte::Const(_) | Byte::Input { .. } => self.insert(g),
            Byte::Add(mut lhs, mut rhs) => {
                if let Byte::Const(a) = g[lhs] {
                    if let Byte::Const(b) = g[rhs] {
                        return Byte::Const(a.wrapping_add(b)).insert(g);
                    }
                    mem::swap(&mut lhs, &mut rhs);
                }
                match (&g[lhs], &g[rhs]) {
                    (_, Byte::Const(0)) => lhs,
                    (_, _) if lhs == rhs => Byte::Mul(lhs, Byte::Const(2).insert(g)).idealize(g),
                    (_, &Byte::Add(b, c)) => {
                        Byte::Add(Byte::Add(lhs, b).idealize(g), c).idealize(g)
                    }
                    (&Byte::Add(a, b), _) => match (&g[b], &g[rhs]) {
                        (&Byte::Const(b), &Byte::Const(c)) => {
                            Byte::Add(a, Byte::Const(b.wrapping_add(c)).insert(g)).idealize(g)
                        }
                        (&Byte::Const(_), _) => {
                            Byte::Add(Byte::Add(a, rhs).idealize(g), b).idealize(g)
                        }
                        _ => Byte::Add(lhs, rhs).insert(g),
                    },
                    _ => Byte::Add(lhs, rhs).insert(g),
                }
            }
            Byte::Mul(mut lhs, mut rhs) => {
                if let Byte::Const(a) = g[lhs] {
                    if let Byte::Const(b) = g[rhs] {
                        return Byte::Const(a.wrapping_mul(b)).insert(g);
                    }
                    mem::swap(&mut lhs, &mut rhs);
                }
                match (&g[lhs], &g[rhs]) {
                    (_, Byte::Const(1)) => lhs,
                    (_, Byte::Const(0)) => Byte::Const(0).insert(g),
                    (_, &Byte::Mul(b, c)) => {
                        Byte::Mul(Byte::Mul(lhs, b).idealize(g), c).idealize(g)
                    }
                    (&Byte::Mul(a, b), _) => match (&g[b], &g[rhs]) {
                        (&Byte::Const(b), &Byte::Const(c)) => {
                            Byte::Mul(a, Byte::Const(b.wrapping_mul(c)).insert(g)).idealize(g)
                        }
                        (&Byte::Const(_), _) => {
                            Byte::Mul(Byte::Mul(a, rhs).idealize(g), b).idealize(g)
                        }
                        _ => Byte::Mul(lhs, rhs).insert(g),
                    },
                    _ => Byte::Mul(lhs, rhs).insert(g),
                }
            }
        }
    }

    /// Gets or inserts this node into a graph and returns its ID.
    #[inline]
    pub fn insert(self, g: &mut Graph) -> ByteId {
        g.insert_byte(self)
    }
}

impl NodeRef<'_, ByteId> {
    /// Returns whether this node references a cell besides at the given offset.
    pub fn references_other(&self, offset: isize) -> bool {
        match *self.node() {
            Byte::Copy(offset2) => offset2 != offset,
            Byte::Const(_) | Byte::Input { .. } => false,
            Byte::Add(lhs, rhs) | Byte::Mul(lhs, rhs) => {
                lhs.get(self.graph()).references_other(offset)
                    || rhs.get(self.graph()).references_other(offset)
            }
        }
    }
}

impl ByteId {
    pub fn rebase(&self, bb: &mut BasicBlock, g: &mut Graph) -> ByteId {
        match g[*self] {
            Byte::Copy(offset) => bb.cell(bb.offset() + offset, g),
            Byte::Const(c) => Byte::Const(c).insert(g),
            Byte::Input { id } => Byte::Input {
                id: id + bb.inputs(),
            }
            .insert(g),
            Byte::Add(lhs, rhs) => Byte::Add(lhs.rebase(bb, g), rhs.rebase(bb, g)).idealize(g),
            Byte::Mul(lhs, rhs) => Byte::Mul(lhs.rebase(bb, g), rhs.rebase(bb, g)).idealize(g),
        }
    }
}

impl Array {
    /// Gets or inserts this node into a graph and returns its ID.
    #[inline]
    pub fn insert(self, g: &mut Graph) -> ArrayId {
        g.insert_array(self)
    }
}

impl ArrayId {
    pub fn rebase(&self, bb: &mut BasicBlock, g: &mut Graph) -> ArrayId {
        let mut array = g[*self].clone();
        for e in &mut array.elements {
            *e = e.rebase(bb, g);
        }
        array.insert(g)
    }
}

impl NodeId {
    pub fn rebase(&self, bb: &mut BasicBlock, g: &mut Graph) -> NodeId {
        if let Some(id) = self.as_byte_id(g) {
            id.rebase(bb, g).as_node_id()
        } else if let Some(id) = self.as_array_id(g) {
            id.rebase(bb, g).as_node_id()
        } else {
            unreachable!();
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{graph::Graph, node::Byte};

    #[test]
    fn idealize_add() {
        let g = &mut Graph::new();
        let x = Byte::Copy(0).idealize(g);
        let y = Byte::Copy(2).idealize(g);
        let add = Byte::Add(
            Byte::Add(x, Byte::Const(1).idealize(g)).idealize(g),
            Byte::Add(y, Byte::Const(3).idealize(g)).idealize(g),
        )
        .idealize(g);
        let expected = Byte::Add(Byte::Add(x, y).insert(g), Byte::Const(4).insert(g)).insert(g);
        assert_eq!(g.get(add), g.get(expected));
    }
}
