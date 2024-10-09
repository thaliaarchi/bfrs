use crate::{
    block::{Block, Effect},
    cfg::Cfg,
    egraph::{Graph, NodeId},
    node::{BlockId, Node},
};

impl Cfg {
    /// Replaces copies with their definition in the preceding block, if the
    /// definition is a constant.
    pub fn opt_copy_const(&mut self, pred: Option<&Block>, g: &mut Graph) {
        match self {
            Cfg::Block(block) => {
                if let Some(pred) = pred {
                    block.copy_const(pred, g);
                }
            }
            Cfg::Seq(seq) => {
                let mut pred = pred;
                for cfg in seq.as_slice_mut() {
                    cfg.opt_copy_const(pred, g);
                    if let Cfg::Block(block) = cfg {
                        pred = Some(block);
                    } else {
                        pred = None;
                    }
                }
                self.flatten(g);
            }
            Cfg::Loop(cfg) => cfg.opt_copy_const(None, g),
            Cfg::If(cfg_then) => cfg_then.opt_copy_const(pred, g),
        }
    }
}

impl Block {
    /// Replaces copies with their definition in the preceding block, if the
    /// definition is a constant.
    pub fn copy_const(&mut self, pred: &Block, g: &mut Graph) {
        let curr = self.id;
        self.iter_memory_mut(g, |_, cell, a| Some(cell.copy_const(curr, pred, a)));
        for effect in &mut self.effects {
            if let Effect::Output(values) = effect {
                for value in values {
                    *value = value.copy_const(curr, pred, g);
                }
            }
        }
    }
}

impl NodeId {
    /// Copes the node, with copies replaced with their definition in the
    /// preceding block, if the definition is a constant.
    fn copy_const(self, curr: BlockId, pred: &Block, g: &mut Graph) -> Self {
        match g[self] {
            Node::Copy(offset, block) if block == curr => pred
                .get_cell(pred.offset + offset)
                .filter(|&cell| matches!(g[cell], Node::Const(_)))
                .unwrap_or(self),
            Node::Copy(..) | Node::Const(_) | Node::Input(_) => self,
            Node::Add(lhs, rhs) => {
                let lhs = lhs.copy_const(curr, pred, g);
                let rhs = rhs.copy_const(curr, pred, g);
                Node::Add(lhs, rhs).insert(g)
            }
            Node::Mul(lhs, rhs) => {
                let lhs = lhs.copy_const(curr, pred, g);
                let rhs = rhs.copy_const(curr, pred, g);
                Node::Mul(lhs, rhs).insert(g)
            }
        }
    }
}
