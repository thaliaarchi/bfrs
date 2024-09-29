use std::{cmp::Ordering, collections::BTreeSet, isize, mem, usize};

use crate::{
    graph::{hash_arena::ArenaRef, Graph, NodeId},
    region::Region,
};

// TODO:
// - Make a Bool node type to replace Condition.
// - Reuse scratch sets for ordering Byte.

/// A node in a graph.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Node {
    // Control flow.
    /// The root of the IR.
    Root { blocks: Vec<NodeId> },
    /// A basic block of non-branching instructions.
    BasicBlock(Region),
    /// Loop while some condition is true.
    Loop {
        /// Loop condition.
        condition: Condition,
        /// The contained blocks.
        body: Vec<NodeId>,
    },

    // Byte values.
    /// Copy the byte from the cell at the offset.
    Copy(isize),
    /// A constant byte.
    Const(u8),
    /// A byte read from the user.
    Input { id: usize },
    /// Addition of two bytes.
    Add(NodeId, NodeId),
    /// Multiplication of two bytes.
    Mul(NodeId, NodeId),

    // Array value.
    /// An array with static size and dynamic elements.
    Array(Vec<NodeId>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeType {
    Control,
    Byte,
    Array,
}

/// A loop condition.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Condition {
    /// Loop while the current cell is non-zero.
    WhileNonZero,
    /// Execute if the current cell is non-zero.
    IfNonZero,
    /// Loop a fixed number of times. The value must be a byte.
    Count(NodeId),
}

impl Node {
    /// Inserts this node into the graph and transforms it to its ideal
    /// representation.
    pub fn idealize(self, g: &Graph) -> NodeId {
        match self {
            Node::Add(mut lhs, mut rhs) => {
                let (mut lhs_ref, mut rhs_ref) = (g.get(lhs), g.get(rhs));
                if let Node::Add(b, c) = *rhs_ref {
                    if let Node::Add(..) = *lhs_ref {
                        return Node::Add(Node::Add(lhs, b).idealize(g), c).idealize(g);
                    }
                    mem::swap(&mut lhs, &mut rhs);
                    mem::swap(&mut lhs_ref, &mut rhs_ref);
                }
                let (tail, head) = match *lhs_ref {
                    Node::Add(a, b) => (Some(a), b),
                    _ => (None, lhs),
                };
                let head_ref = g.get(head);
                let (res, idealize) = match (&*head_ref, &*rhs_ref) {
                    (&Node::Const(a), &Node::Const(b)) => {
                        (Node::Const(a.wrapping_add(b)).insert(g), true)
                    }
                    (_, Node::Const(0)) => (head, false),
                    (Node::Const(0), _) => (rhs, true),
                    (Node::Const(_), _) => {
                        if let Some(tail) = tail {
                            return Node::Add(Node::Add(tail, rhs).idealize(g), head).idealize(g);
                        } else {
                            return Node::Add(rhs, head).insert(g);
                        }
                    }
                    _ if head == rhs => {
                        (Node::Mul(head, Node::Const(2).insert(g)).idealize(g), true)
                    }
                    (&Node::Mul(a, b), _) if a == rhs => {
                        let n = Node::Add(b, Node::Const(1).insert(g)).idealize(g);
                        (Node::Mul(a, n).idealize(g), true)
                    }
                    (_, &Node::Mul(b, c)) if b == head => {
                        let n = Node::Add(c, Node::Const(1).insert(g)).idealize(g);
                        (Node::Mul(b, n).idealize(g), true)
                    }
                    _ if head_ref.cmp_by_variable_order(&rhs_ref).is_gt() => {
                        if let Some(tail) = tail {
                            return Node::Add(Node::Add(tail, rhs).idealize(g), head).idealize(g);
                        } else {
                            return Node::Add(rhs, head).insert(g);
                        }
                    }
                    _ => return Node::Add(lhs, rhs).insert(g),
                };
                if let Some(tail) = tail {
                    if res == head {
                        lhs
                    } else if idealize {
                        Node::Add(tail, res).idealize(g)
                    } else {
                        Node::Add(tail, res).insert(g)
                    }
                } else {
                    res
                }
            }
            Node::Mul(mut lhs, mut rhs) => {
                let (mut lhs_ref, mut rhs_ref) = (g.get(lhs), g.get(rhs));
                if let Node::Mul(b, c) = *rhs_ref {
                    if let Node::Mul(..) = *lhs_ref {
                        return Node::Mul(Node::Mul(lhs, b).idealize(g), c).idealize(g);
                    }
                    mem::swap(&mut lhs, &mut rhs);
                    mem::swap(&mut lhs_ref, &mut rhs_ref);
                }
                let (tail, head) = match *lhs_ref {
                    Node::Mul(a, b) => (Some(a), b),
                    _ => (None, lhs),
                };
                let head_ref = g.get(head);
                let (res, idealize) = match (&*head_ref, &*rhs_ref) {
                    (&Node::Const(a), &Node::Const(b)) => {
                        (Node::Const(a.wrapping_mul(b)).insert(g), true)
                    }
                    (_, Node::Const(1)) => (head, false),
                    (Node::Const(1), _) => (rhs, true),
                    (_, Node::Const(0)) | (Node::Const(0), _) => return Node::Const(0).insert(g),
                    (Node::Const(_), _) => {
                        if let Some(tail) = tail {
                            return Node::Mul(Node::Mul(tail, rhs).idealize(g), head).idealize(g);
                        } else {
                            return Node::Mul(rhs, head).insert(g);
                        }
                    }
                    _ if head_ref.cmp_by_variable_order(&rhs_ref).is_gt() => {
                        if let Some(tail) = tail {
                            return Node::Mul(Node::Mul(tail, rhs).idealize(g), head).idealize(g);
                        } else {
                            return Node::Mul(rhs, head).insert(g);
                        }
                    }
                    _ => return Node::Mul(lhs, rhs).insert(g),
                };
                if let Some(tail) = tail {
                    if res == head {
                        lhs
                    } else if idealize {
                        Node::Mul(tail, res).idealize(g)
                    } else {
                        Node::Mul(tail, res).insert(g)
                    }
                } else {
                    res
                }
            }
            _ => self.insert(g),
        }
    }

    pub fn ty(&self) -> NodeType {
        match self {
            Node::Root { .. } | Node::BasicBlock(_) | Node::Loop { .. } => NodeType::Control,
            Node::Copy(_) | Node::Const(_) | Node::Input { .. } | Node::Add(..) | Node::Mul(..) => {
                NodeType::Byte
            }
            Node::Array(_) => NodeType::Array,
        }
    }
}

impl ArenaRef<'_, Node> {
    /// Returns whether this node references a cell besides at the given offset.
    pub fn references_other(&self, offset: isize) -> bool {
        match *self.value() {
            Node::Root { .. } | Node::BasicBlock(_) | Node::Loop { .. } => {
                panic!("unexpected control node")
            }
            Node::Copy(offset2) => offset2 != offset,
            Node::Const(_) | Node::Input { .. } => false,
            Node::Add(lhs, rhs) | Node::Mul(lhs, rhs) => {
                self.get(lhs).references_other(offset) || self.get(rhs).references_other(offset)
            }
            Node::Array(ref elements) => elements
                .iter()
                .any(|&e| self.get(e).references_other(offset)),
        }
    }

    /// Orders two `Node` nodes by variable ordering, i.e., the contained `Copy`
    /// offsets and `Input` IDs.
    pub fn cmp_by_variable_order(&self, other: &Self) -> Ordering {
        match (self.value(), other.value()) {
            (Node::Const(a), Node::Const(b)) => a.cmp(b),
            (_, Node::Const(_)) => Ordering::Less,
            (Node::Const(_), _) => Ordering::Greater,
            (Node::Copy(a), Node::Copy(b)) => a.cmp(b),
            (Node::Input { id: a }, Node::Input { id: b }) => a.cmp(b),
            (Node::Copy(_), Node::Input { .. }) => Ordering::Less,
            (Node::Input { .. }, Node::Copy(_)) => Ordering::Greater,
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
        match *self.value() {
            Node::Root { .. } | Node::BasicBlock(_) | Node::Loop { .. } => {
                panic!("unexpected control node")
            }
            Node::Copy(offset) => {
                debug_assert!(offset != isize::MAX);
                *min_offset = offset.min(*min_offset);
            }
            Node::Const(_) => {}
            Node::Input { id } => {
                debug_assert!(id != usize::MAX);
                *min_input = id.min(*min_input);
            }
            Node::Add(lhs, rhs) => {
                self.get(lhs).min_terms_(min_offset, min_input);
                self.get(rhs).min_terms_(min_offset, min_input);
            }
            Node::Mul(lhs, rhs) => {
                self.get(lhs).min_terms_(min_offset, min_input);
                self.get(rhs).min_terms_(min_offset, min_input);
            }
            Node::Array(ref elements) => {
                for &e in elements {
                    self.get(e).min_terms_(min_offset, min_input);
                }
            }
        }
    }

    fn offsets(&self, offsets: &mut BTreeSet<isize>) {
        match *self.value() {
            Node::Root { .. } | Node::BasicBlock(_) | Node::Loop { .. } => {
                panic!("unexpected control node")
            }
            Node::Copy(offset) => {
                offsets.insert(offset);
            }
            Node::Const(_) | Node::Input { .. } => {}
            Node::Add(lhs, rhs) | Node::Mul(lhs, rhs) => {
                self.get(lhs).offsets(offsets);
                self.get(rhs).offsets(offsets);
            }
            Node::Array(ref elements) => {
                for &e in elements {
                    self.get(e).offsets(offsets);
                }
            }
        }
    }

    fn inputs(&self, inputs: &mut BTreeSet<usize>) {
        match *self.value() {
            Node::Root { .. } | Node::BasicBlock(_) | Node::Loop { .. } => {
                panic!("unexpected control node")
            }
            Node::Copy(_) | Node::Const(_) => {}
            Node::Input { id } => {
                inputs.insert(id);
            }
            Node::Add(lhs, rhs) | Node::Mul(lhs, rhs) => {
                self.get(lhs).inputs(inputs);
                self.get(rhs).inputs(inputs);
            }
            Node::Array(ref elements) => {
                for &e in elements {
                    self.get(e).inputs(inputs);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{graph::Graph, node::Node};

    #[test]
    fn idealize_add() {
        let g = Graph::new();
        let x = Node::Copy(0).idealize(&g);
        let y = Node::Copy(2).idealize(&g);
        let add = Node::Add(
            Node::Add(x, Node::Const(1).idealize(&g)).idealize(&g),
            Node::Add(y, Node::Const(3).idealize(&g)).idealize(&g),
        )
        .idealize(&g);
        let expected = Node::Add(Node::Add(x, y).insert(&g), Node::Const(4).insert(&g)).insert(&g);
        assert_eq!(g.get(add), g.get(expected));
    }
}
