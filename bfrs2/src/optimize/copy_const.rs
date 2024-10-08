use crate::{
    arena::{Arena, NodeId},
    block::{Block, Effect},
    cfg::Cfg,
    node::{BlockId, Node},
};

impl Cfg {
    /// Replaces copies with their definition in the preceding block, if the
    /// definition is a constant.
    pub fn opt_copy_const(&mut self, pred: Option<&Block>, a: &mut Arena) {
        match self {
            Cfg::Block(block) => {
                if let Some(pred) = pred {
                    block.copy_const(pred, a);
                }
            }
            Cfg::Seq(seq) => {
                let mut pred = pred;
                for cfg in seq.as_slice_mut() {
                    cfg.opt_copy_const(pred, a);
                    if let Cfg::Block(block) = cfg {
                        pred = Some(block);
                    } else {
                        pred = None;
                    }
                }
                self.flatten(a);
            }
            Cfg::Loop(cfg) => cfg.opt_copy_const(None, a),
            Cfg::If(cfg_then) => cfg_then.opt_copy_const(pred, a),
        }
    }
}

impl Block {
    /// Replaces copies with their definition in the preceding block, if the
    /// definition is a constant.
    pub fn copy_const(&mut self, pred: &Block, a: &mut Arena) {
        let curr = self.id;
        for (_, cell) in self.iter_memory_mut() {
            *cell = cell.copy_const(curr, pred, a);
        }
        for effect in &mut self.effects {
            if let Effect::Output(values) = effect {
                for value in values {
                    *value = value.copy_const(curr, pred, a);
                }
            }
        }
    }
}

impl NodeId {
    /// Copes the node, with copies replaced with their definition in the
    /// preceding block, if the definition is a constant.
    fn copy_const(self, curr: BlockId, pred: &Block, a: &mut Arena) -> Self {
        match a[self] {
            Node::Copy(offset, block) if block == curr => pred
                .get_cell(pred.offset + offset)
                .filter(|&cell| matches!(a[cell], Node::Const(_)))
                .unwrap_or(self),
            Node::Copy(..) | Node::Const(_) | Node::Input(_) => self,
            Node::Add(lhs, rhs) => {
                let lhs = lhs.copy_const(curr, pred, a);
                let rhs = rhs.copy_const(curr, pred, a);
                Node::Add(lhs, rhs).insert(a)
            }
            Node::Mul(lhs, rhs) => {
                let lhs = lhs.copy_const(curr, pred, a);
                let rhs = rhs.copy_const(curr, pred, a);
                Node::Mul(lhs, rhs).insert(a)
            }
        }
    }
}
