use std::{cmp::Ordering, collections::BTreeSet, isize, mem, usize};

use crate::{
    graph::{ArrayId, ByteId, Graph, NodeId, NodeRef},
    ir::BasicBlock,
};

// TODO:
// - Reuse scratch sets for ordering Byte.

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
                        (Byte::Const(a.wrapping_add(b)).insert(g), true)
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
                    (&Byte::Mul(a, b), _) if a == rhs => {
                        let n = Byte::Add(b, Byte::Const(1).insert(g)).idealize(g);
                        (Byte::Mul(a, n).idealize(g), true)
                    }
                    (_, &Byte::Mul(b, c)) if b == head => {
                        let n = Byte::Add(c, Byte::Const(1).insert(g)).idealize(g);
                        (Byte::Mul(b, n).idealize(g), true)
                    }
                    _ if head.get(g).cmp_by_variable_order(rhs.get(g)).is_gt() => {
                        if let Some(tail) = tail {
                            return Byte::Add(Byte::Add(tail, rhs).idealize(g), head).idealize(g);
                        } else {
                            return Byte::Add(rhs, head).insert(g);
                        }
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
                        (Byte::Const(a.wrapping_mul(b)).insert(g), true)
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
                    _ if head.get(g).cmp_by_variable_order(rhs.get(g)).is_gt() => {
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

    /// Orders two `Byte` nodes by variable ordering, i.e., the contained `Copy`
    /// offsets and `Input` IDs.
    pub fn cmp_by_variable_order(&self, other: Self) -> Ordering {
        match (self.node(), other.node()) {
            (Byte::Const(a), Byte::Const(b)) => a.cmp(b),
            (_, Byte::Const(_)) => Ordering::Less,
            (Byte::Const(_), _) => Ordering::Greater,
            (Byte::Copy(a), Byte::Copy(b)) => a.cmp(b),
            (Byte::Input { id: a }, Byte::Input { id: b }) => a.cmp(b),
            (Byte::Copy(_), Byte::Input { .. }) => Ordering::Less,
            (Byte::Input { .. }, Byte::Copy(_)) => Ordering::Greater,
            _ => {
                // The general case. First, try a scan which tracks only the
                // minimums to avoid most allocations.
                let (min_offset1, min_input1) = self.min_terms();
                let (min_offset2, min_input2) = other.min_terms();
                if min_offset1 != min_offset2 {
                    return min_offset1.cmp(&min_offset2);
                }
                // TODO: Reuse these scratch sets.
                let mut offsets1 = BTreeSet::new();
                let mut offsets2 = BTreeSet::new();
                self.offsets(&mut offsets1);
                other.offsets(&mut offsets2);
                for (offset1, offset2) in offsets1.iter().zip(offsets2.iter()) {
                    if offset1 != offset2 {
                        return offset1.cmp(offset2);
                    }
                }
                if offsets1.len() != offsets2.len() {
                    return offsets1.len().cmp(&offsets2.len()).reverse();
                }
                if min_input1 != min_input2 {
                    return min_input1.cmp(&min_input2);
                }
                let mut inputs1 = BTreeSet::new();
                let mut inputs2 = BTreeSet::new();
                self.inputs(&mut inputs1);
                other.inputs(&mut inputs2);
                for (input1, input2) in inputs1.iter().zip(inputs2.iter()) {
                    if input1 != input2 {
                        return input1.cmp(input2);
                    }
                }
                inputs1.len().cmp(&inputs2.len()).reverse()
            }
        }
    }

    /// Computes the least offset for a `Copy` and the least ID for an `Input`
    /// in this expression, or the maximum integer value if none is found.
    fn min_terms(&self) -> (isize, usize) {
        let mut min_offset = isize::MAX;
        let mut min_input = usize::MAX;
        self.min_terms_(&mut min_offset, &mut min_input);
        (min_offset, min_input)
    }

    fn min_terms_(&self, min_offset: &mut isize, min_input: &mut usize) {
        match *self.node() {
            Byte::Copy(offset) => {
                debug_assert!(offset != isize::MAX);
                *min_offset = offset.min(*min_offset);
            }
            Byte::Const(_) => {}
            Byte::Input { id } => {
                debug_assert!(id != usize::MAX);
                *min_input = id.min(*min_input);
            }
            Byte::Add(lhs, rhs) => {
                self.graph().get(lhs).min_terms_(min_offset, min_input);
                self.graph().get(rhs).min_terms_(min_offset, min_input);
            }
            Byte::Mul(lhs, rhs) => {
                self.graph().get(lhs).min_terms_(min_offset, min_input);
                self.graph().get(rhs).min_terms_(min_offset, min_input);
            }
        }
    }

    fn offsets(&self, offsets: &mut BTreeSet<isize>) {
        match *self.node() {
            Byte::Copy(offset) => {
                offsets.insert(offset);
            }
            Byte::Const(_) | Byte::Input { .. } => {}
            Byte::Add(lhs, rhs) | Byte::Mul(lhs, rhs) => {
                self.graph().get(lhs).offsets(offsets);
                self.graph().get(rhs).offsets(offsets);
            }
        }
    }

    fn inputs(&self, inputs: &mut BTreeSet<usize>) {
        match *self.node() {
            Byte::Copy(_) | Byte::Const(_) => {}
            Byte::Input { id } => {
                inputs.insert(id);
            }
            Byte::Add(lhs, rhs) | Byte::Mul(lhs, rhs) => {
                self.graph().get(lhs).inputs(inputs);
                self.graph().get(rhs).inputs(inputs);
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
