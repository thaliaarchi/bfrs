use std::mem;

use crate::{
    block::Block,
    cfg::{Cfg, Seq},
    egraph::Graph,
    node::Offset,
};

// TODO:
// - Investigate peeling looped sequences.

impl Cfg {
    pub fn opt_peel(&mut self, g: &mut Graph) {
        match self {
            Cfg::Block(_) => {}
            Cfg::Seq(seq) => {
                seq.iter_mut().for_each(|cfg| cfg.opt_peel(g));
                self.flatten(g);
            }
            Cfg::Loop(cfg) => {
                if let Cfg::Block(block) = cfg.as_ref() {
                    if block.offset == Offset(0) && block.has_invariant_stores(g) {
                        let mut tail = block.clone_fresh(g);
                        tail.remove_invariant_stores(block, g);
                        tail.copy_const(block, g);
                        let mut tail = Cfg::Loop(Box::new(Cfg::Block(tail)));
                        tail.opt_closed_form_add(g);
                        tail.opt_peel(g);

                        let Cfg::Loop(peeled) = mem::replace(self, Cfg::empty()) else {
                            unreachable!();
                        };
                        let body = Seq::from_iter([*peeled, tail], g).into_cfg();
                        *self = Cfg::If(Box::new(body));
                        return;
                    }
                }
                cfg.opt_peel(g);
            }
            Cfg::If(cfg_then) => {
                cfg_then.opt_peel(g);
            }
        }
    }
}

impl Block {
    /// Returns whether at least one value in the block stores a value that
    /// would not change after another iteration.
    fn has_invariant_stores(&self, g: &Graph) -> bool {
        for (_, cell) in self.iter_memory() {
            if !g.get(cell).reads_from(self, self.id) {
                return true;
            }
        }
        false
    }

    /// Removes any values stored in the block that would not change after
    /// another iteration.
    fn remove_invariant_stores(&mut self, original: &Block, g: &mut Graph) {
        let curr = self.id;
        self.iter_memory_mut(g, |_, cell, a| {
            if !a.get(cell).reads_from(original, curr) {
                None
            } else {
                Some(cell)
            }
        });
    }
}
