use crate::{
    arena::NodeRef,
    block::{Block, Effect},
    node::{BlockId, Node, Offset},
};

impl NodeRef<'_> {
    /// Returns whether this value is loop-invariant or an add-assign with a
    /// loop-invariant value.
    pub fn is_add_assign(&self, offset: Offset, block: BlockId) -> bool {
        if let Node::Add(lhs, rhs) = *self.node() {
            *self.get(lhs) == Node::Copy(offset, block) && self.get(rhs).is_loop_invariant()
        } else {
            self.is_loop_invariant()
        }
    }

    /// Returns whether this value is loop-invariant.
    pub fn is_loop_invariant(&self) -> bool {
        match *self.node() {
            Node::Copy(..) | Node::Input(_) => false,
            Node::Const(_) => true,
            Node::Add(lhs, rhs) | Node::Mul(lhs, rhs) => {
                self.get(lhs).is_loop_invariant() && self.get(rhs).is_loop_invariant()
            }
        }
    }

    /// Returns whether this value reads from the block.
    pub fn reads_from(&self, block: &Block) -> bool {
        match *self.node() {
            Node::Copy(offset, block_id) => {
                block_id == block.id && block.get_cell(offset).is_some()
            }
            Node::Const(_) => false,
            Node::Input(_) => true,
            Node::Add(lhs, rhs) | Node::Mul(lhs, rhs) => {
                self.get(lhs).reads_from(block) || self.get(rhs).reads_from(block)
            }
        }
    }
}

impl Block {
    /// Reports whether the block has no I/O effects and, if so, whether it has
    /// guards. Returns `None` when the block has I/O, `Some(false)` when the
    /// block has no I/O and no guards, and `Some(true)` when the block has no
    /// I/O and has guards.
    pub fn is_pure(&self) -> Option<bool> {
        let mut has_guards = false;
        for effect in &self.effects {
            match effect {
                Effect::GuardShift(_) => has_guards = true,
                _ => return None,
            }
        }
        Some(has_guards)
    }
}
