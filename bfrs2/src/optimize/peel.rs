use std::mem;

use crate::{arena::Arena, block::Block, cfg::Cfg, node::Offset};

// TODO:
// - Investigate peeling looped sequences.

impl Cfg {
    pub fn opt_peel(&mut self, a: &mut Arena) {
        match self {
            Cfg::Block(_) => {}
            Cfg::Seq(seq) => {
                for cfg in seq {
                    cfg.opt_peel(a);
                }
                // TODO: Concatenate adjacent basic blocks.
            }
            Cfg::Loop(cfg) => {
                cfg.opt_peel(a);
                if let Cfg::Block(block) = cfg.as_ref() {
                    if block.offset != Offset(0) || !block.has_invariant_stores(a) {
                        return;
                    }
                    let mut tail = block.clone_fresh(a);
                    tail.remove_invariant_stores(a);
                    let mut tail = Cfg::Loop(Box::new(Cfg::Block(tail)));
                    tail.opt_peel(a);

                    let Cfg::Loop(peeled) = mem::replace(self, Cfg::empty()) else {
                        unreachable!();
                    };
                    let body = Cfg::Seq(vec![*peeled, tail]);
                    // TODO: Concatenate adjacent basic blocks.
                    *self = Cfg::If(Box::new(body));
                }
            }
            Cfg::If(cfg_then) => {
                cfg_then.opt_peel(a);
            }
        }
    }
}

impl Block {
    /// Returns whether at least one value in the block stores a value that
    /// would not change after another iteration.
    fn has_invariant_stores(&self, a: &Arena) -> bool {
        for (_, cell) in self.iter_memory() {
            if !a.get(cell).reads_from(self) {
                return true;
            }
        }
        false
    }

    /// Removes any values stored in the block that would not change after
    /// another iteration.
    fn remove_invariant_stores(&mut self, a: &Arena) {
        for i in 0..self.memory.len() {
            if self.memory[i].is_some_and(|cell| !a.get(cell).reads_from(self)) {
                self.memory[i] = None;
            }
        }
    }
}
