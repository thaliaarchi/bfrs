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
                if let Byte::Add(b, c) = g[rhs] {
                    if let Byte::Add(..) = g[lhs] {
                        return Byte::Add(Byte::Add(lhs, b).idealize(g), c).idealize(g);
                    }
                    mem::swap(&mut lhs, &mut rhs);
                }
                let (tail, head) = match g[lhs] {
                    Byte::Add(a, b) => (Some(a), b),
                    _ => (None, lhs),
                };
                let (res, idealize) = match (&g[head], &g[rhs]) {
                    (&Byte::Const(a), &Byte::Const(b)) => {
                        (Byte::Const(a.wrapping_add(b)).insert(g), false)
                    }
                    (_, Byte::Const(0)) => (head, false),
                    (Byte::Const(0), _) => (rhs, true),
                    (Byte::Const(_), _) => {
                        if let Some(tail) = tail {
                            return Byte::Add(Byte::Add(tail, rhs).idealize(g), head).idealize(g);
                        } else {
                            return Byte::Add(rhs, head).insert(g);
                        }
                    }
                    _ if head == rhs => {
                        (Byte::Mul(head, Byte::Const(2).insert(g)).idealize(g), true)
                    }
                    (&Byte::Mul(a, b), _) if a == rhs && matches!(g[b], Byte::Const(255)) => {
                        (Byte::Const(0).insert(g), false)
                    }
                    (_, &Byte::Mul(b, c)) if b == head && matches!(g[c], Byte::Const(255)) => {
                        (Byte::Const(0).insert(g), false)
                    }
                    _ => return Byte::Add(lhs, rhs).insert(g),
                };
                if let Some(tail) = tail {
                    if res == head {
                        lhs
                    } else if idealize {
                        Byte::Add(tail, res).idealize(g)
                    } else {
                        Byte::Add(tail, res).insert(g)
                    }
                } else {
                    res
                }
            }
            Byte::Mul(mut lhs, mut rhs) => {
                if let Byte::Mul(b, c) = g[rhs] {
                    if let Byte::Mul(..) = g[lhs] {
                        return Byte::Mul(Byte::Mul(lhs, b).idealize(g), c).idealize(g);
                    }
                    mem::swap(&mut lhs, &mut rhs);
                }
                let (tail, head) = match g[lhs] {
                    Byte::Mul(a, b) => (Some(a), b),
                    _ => (None, lhs),
                };
                let (res, idealize) = match (&g[head], &g[rhs]) {
                    (&Byte::Const(a), &Byte::Const(b)) => {
                        (Byte::Const(a.wrapping_mul(b)).insert(g), false)
                    }
                    (_, Byte::Const(1)) => (head, false),
                    (Byte::Const(1), _) => (rhs, true),
                    (_, Byte::Const(0)) | (Byte::Const(0), _) => return Byte::Const(0).insert(g),
                    (Byte::Const(_), _) => {
                        if let Some(tail) = tail {
                            return Byte::Mul(Byte::Mul(tail, rhs).idealize(g), head).idealize(g);
                        } else {
                            return Byte::Mul(rhs, head).insert(g);
                        }
                    }
                    _ => return Byte::Mul(lhs, rhs).insert(g),
                };
                if let Some(tail) = tail {
                    if res == head {
                        lhs
                    } else if idealize {
                        Byte::Mul(tail, res).idealize(g)
                    } else {
                        Byte::Mul(tail, res).insert(g)
                    }
                } else {
                    res
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
