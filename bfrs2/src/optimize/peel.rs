use std::mem;

use crate::{
    arena::Arena,
    block::Block,
    cfg::{Cfg, Seq},
    node::Offset,
};

// TODO:
// - Investigate peeling looped sequences.

impl Cfg {
    pub fn opt_peel(&mut self, a: &mut Arena) {
        match self {
            Cfg::Block(_) => {}
            Cfg::Seq(seq) => {
                seq.for_each(a, |cfg, a| cfg.opt_peel(a));
                self.flatten();
            }
            Cfg::Loop(cfg) => {
                if let Cfg::Block(block) = cfg.as_ref() {
                    if block.offset == Offset(0) && block.has_invariant_stores(a) {
                        let mut tail = block.clone_fresh(a);
                        tail.remove_invariant_stores(a);
                        let tail = Cfg::Loop(Box::new(Cfg::Block(tail)));

                        let Cfg::Loop(peeled) = mem::replace(self, Cfg::empty()) else {
                            unreachable!();
                        };
                        let body = Seq::from_iter([*peeled, tail], a).into_cfg();
                        *self = Cfg::If(Box::new(body));
                        self.opt_peel(a);
                        return;
                    }
                }
                cfg.opt_peel(a);
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
