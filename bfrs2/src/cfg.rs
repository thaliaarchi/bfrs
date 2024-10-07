use std::mem;

use crate::{arena::Arena, block::Block};

/// The control-flow graph of a program.
#[derive(Clone, Debug)]
pub enum Cfg {
    /// Basic block.
    Block(Block),
    /// Sequence.
    Seq(Vec<Cfg>),
    /// Loop while the current cell is non-zero.
    Loop(Box<Cfg>),
    /// If the current cell is non-zero.
    If(Box<Cfg>),
}

impl Cfg {
    /// Creates a CFG with no effect.
    pub fn empty() -> Self {
        Cfg::Seq(Vec::new())
    }

    /// Constructs a sequence, which flattens any top-level sequences.
    pub fn seq(seq: Vec<Cfg>) -> Self {
        if !seq.iter().any(|cfg| matches!(cfg, Cfg::Seq(_))) {
            return Cfg::Seq(seq);
        }
        let mut flattened = Vec::new();
        for cfg in seq {
            match cfg {
                Cfg::Seq(seq) => flattened.extend(seq.into_iter()),
                _ => flattened.push(cfg),
            }
        }
        if flattened.len() == 1 {
            flattened.pop().unwrap()
        } else {
            Cfg::Seq(flattened)
        }
    }

    /// Concatenates adjacent basic blocks in the sequence.
    pub fn concat_adjacent_blocks(&mut self, a: &mut Arena) {
        let Cfg::Seq(seq) = self else { return };
        seq.dedup_by(|cfg2, cfg1| match (cfg1, cfg2) {
            (Cfg::Block(block1), Cfg::Block(block2)) => {
                block1.concat(block2, a);
                true
            }
            _ => false,
        });
        if seq.len() == 1 {
            let Cfg::Seq(mut seq) = mem::replace(self, Cfg::empty()) else {
                unreachable!();
            };
            *self = seq.pop().unwrap();
        }
    }
}
