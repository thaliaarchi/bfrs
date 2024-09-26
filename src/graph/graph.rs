use crate::{
    graph::{
        arena::Id,
        hash_arena::{ArenaRef, HashArena},
    },
    node::Node,
};
#[cfg(debug_assertions)]
use crate::{
    node::{Condition, NodeType},
    region::Effect,
};

/// A graph of unique nodes, structured as an arena.
///
/// # Safety
///
/// It is undefined behavior to use a `NodeId` in any graph other than the one
/// which created it.
pub type Graph = HashArena<Node>;

/// The ID of a node in a graph.
pub type NodeId = Id<Node>;

#[cfg(debug_assertions)]
impl Graph {
    fn assert_byte(&self, id: NodeId) {
        self.assert_id(id);
        assert_eq!(
            unsafe { self.get_unchecked(id) }.ty(),
            NodeType::Byte,
            "node is not a byte",
        );
    }

    fn assert_node(&self, node: &Node) {
        match node {
            Node::Root { blocks } | Node::Loop { body: blocks, .. } => {
                for &block in blocks {
                    self.assert_id(block);
                    assert!(
                        matches!(
                            unsafe { self.get_unchecked(block) },
                            Node::BasicBlock(_) | Node::Loop { .. },
                        ),
                        "node is not a control node",
                    );
                }
                if let Node::Loop { condition, .. } = node {
                    match *condition {
                        Condition::WhileNonZero | Condition::IfNonZero => {}
                        Condition::Count(id) => self.assert_byte(id),
                    }
                }
            }
            Node::BasicBlock(region) => {
                for (_, cell) in region.memory.iter() {
                    self.assert_byte(cell);
                }
                for effect in &region.effects {
                    match *effect {
                        Effect::Output(id) => {
                            self.assert_id(id);
                            assert!(
                                matches!(
                                    unsafe { self.get_unchecked(id) }.ty(),
                                    NodeType::Byte | NodeType::Array,
                                ),
                                "node is not a byte or array",
                            );
                        }
                        Effect::Input(id) => self.assert_byte(id),
                        Effect::GuardShift(_) => {}
                    }
                }
            }
            Node::Copy(_) | Node::Const(_) | Node::Input { .. } => {}
            &Node::Add(lhs, rhs) | &Node::Mul(lhs, rhs) => {
                self.assert_byte(lhs);
                self.assert_byte(rhs);
            }
            Node::Array(elements) => {
                for &id in elements {
                    self.assert_byte(id);
                }
            }
        }
    }
}

impl Node {
    /// Gets or inserts this node into a graph and returns its ID.
    #[inline]
    pub fn insert(self, g: &Graph) -> NodeId {
        #[cfg(debug_assertions)]
        g.assert_node(&self);
        match &self {
            Node::Root { .. }
            | Node::BasicBlock(_)
            | Node::Loop { .. }
            | Node::Copy(_)
            | Node::Input { .. } => g.insert_unique(self),
            Node::Const(_) | Node::Add(..) | Node::Mul(..) | Node::Array(_) => g.insert(self),
        }
    }
}

impl<'g> ArenaRef<'g, Node> {
    pub fn graph(&self) -> &'g Graph {
        self.arena()
    }
}

#[cfg(test)]
mod tests {
    use crate::{graph::Graph, node::Node};

    #[test]
    fn insert_unique() {
        let g = Graph::new();
        let id0 = Node::Copy(0).insert(&g);
        let id1 = Node::Const(1).insert(&g);
        let id2 = Node::Add(id0, id1).insert(&g);
        let id0b = Node::Copy(0).insert(&g);
        assert_ne!(id0, id0b);
        let id1b = Node::Const(1).insert(&g);
        let id2b = Node::Add(id0, id1b).insert(&g);
        assert_eq!(id1, id1b);
        assert_eq!(id2, id2b);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn compare_mixed_ids() {
        let g1 = Graph::new();
        let g2 = Graph::new();
        let id1 = Node::Const(1).insert(&g1);
        let id2 = Node::Const(2).insert(&g2);
        assert_eq!(id1.index(), id2.index());
        assert_ne!(id1, id2);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn insert_mixed_ids() {
        let g1 = Graph::new();
        let g2 = Graph::new();
        let id1 = Node::Const(1).insert(&g1);
        let id2 = Node::Const(2).insert(&g1);
        Node::Add(id1, id2).insert(&g2);
    }
}
