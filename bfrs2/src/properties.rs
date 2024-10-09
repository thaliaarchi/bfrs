use crate::{
    block::{Block, Effect},
    egraph::NodeRef,
    node::{BlockId, Node, Offset},
};

impl NodeRef<'_> {
    /// Returns whether this value is loop-invariant or an add-assign with a
    /// loop-invariant value.
    pub fn is_add_assign(&self, offset: Offset, block: &Block) -> bool {
        if let Node::Add(lhs, rhs) = *self.node() {
            *self.get(lhs) == Node::Copy(offset, block.id)
                && !self.get(rhs).reads_from(block, block.id)
        } else {
            !self.reads_from(block, block.id)
        }
    }

    /// Returns whether this value reads from the block.
    pub fn reads_from(&self, block: &Block, copy_from: BlockId) -> bool {
        match *self.node() {
            Node::Copy(offset, block_id) => {
                block_id == copy_from && block.get_cell(offset).is_some()
            }
            Node::Const(_) => false,
            Node::Input(_) => true,
            Node::Add(lhs, rhs) | Node::Mul(lhs, rhs) => {
                self.get(lhs).reads_from(block, copy_from)
                    || self.get(rhs).reads_from(block, copy_from)
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
