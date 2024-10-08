use std::{
    fmt::{self, Debug, Formatter},
    ops::{Deref, DerefMut},
};

use crate::{arena::Arena, block::Block};

/// The control-flow graph of a program.
#[derive(Clone)]
pub enum Cfg {
    /// Basic block.
    Block(Block),
    /// Sequence.
    Seq(Seq),
    /// Loop while the current cell is non-zero.
    Loop(Box<Cfg>),
    /// If the current cell is non-zero.
    If(Box<Cfg>),
}

/// A sequence of control-flow nodes.
#[derive(Clone)]
pub struct Seq {
    cfgs: Vec<Cfg>,
}

impl Cfg {
    /// Creates a CFG with no effect.
    pub fn empty() -> Self {
        Cfg::Seq(Seq::new())
    }

    /// Flattens a 1-element `Seq` into its element.
    pub fn flatten(&mut self, a: &mut Arena) {
        if let Cfg::Seq(seq) = self {
            seq.flatten(a);
            if seq.len() == 1 {
                *self = seq.cfgs.pop().unwrap();
            }
        }
    }
}

impl Debug for Cfg {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Cfg::Block(block) => Debug::fmt(block, f),
            Cfg::Seq(seq) => Debug::fmt(seq, f),
            Cfg::Loop(cfg) => f.debug_tuple("Loop").field(cfg).finish(),
            Cfg::If(cfg) => f.debug_tuple("If").field(cfg).finish(),
        }
    }
}

impl Seq {
    /// Constructs a new, empty sequence.
    pub fn new() -> Self {
        Seq::with_capacity(0)
    }

    /// Constructs a new, empty sequence with the given capacity.
    pub fn with_capacity(capacity: usize) -> Self {
        Seq {
            cfgs: Vec::with_capacity(capacity),
        }
    }

    /// Constructs a sequence from an iterator of `Cfg`s.
    pub fn from_iter(iter: impl IntoIterator<Item = Cfg>, a: &mut Arena) -> Self {
        let iter = iter.into_iter();
        let mut seq = Seq::with_capacity(iter.size_hint().0);
        iter.for_each(|cfg| seq.push(cfg, a));
        seq
    }

    /// Constructs a sequence from externally constructed `Cfg`s. The caller
    /// must guarantee that it is flattened.
    pub fn from_unflattened(seq: Vec<Cfg>) -> Self {
        Seq { cfgs: seq }
    }

    /// Pushes a `Cfg` to the sequence and concanates adjacent blocks and
    /// flattens top-level sequences.
    pub fn push(&mut self, cfg: Cfg, a: &mut Arena) {
        match (self.cfgs.last_mut(), cfg) {
            (Some(Cfg::Block(block1)), Cfg::Block(block2)) => block1.concat(&block2, a),
            (_, Cfg::Seq(seq)) => self.cfgs.extend(seq.cfgs.into_iter()),
            (_, cfg) => self.cfgs.push(cfg),
        }
    }

    /// Concanates adjacent blocks and flattens top-level sequences.
    pub fn flatten(&mut self, a: &mut Arena) {
        let mut has_nested_seq = false;
        let mut flattened_len = self.cfgs.len();
        if let Some(Cfg::Seq(seq)) = self.cfgs.first() {
            has_nested_seq = true;
            flattened_len = flattened_len - 1 + seq.cfgs.len();
        }
        self.cfgs.dedup_by(|cfg2, cfg1| match (cfg1, cfg2) {
            (Cfg::Block(block1), Cfg::Block(block2)) => {
                block1.concat(block2, a);
                true
            }
            (_, Cfg::Seq(seq)) => {
                has_nested_seq = true;
                flattened_len = flattened_len - 1 + seq.cfgs.len();
                false
            }
            _ => false,
        });
        if !has_nested_seq {
            return;
        }
        let mut flattened = Vec::with_capacity(flattened_len);
        for cfg in self.cfgs.drain(..) {
            match cfg {
                Cfg::Seq(inner) => flattened.extend(inner.cfgs.into_iter()),
                _ => flattened.push(cfg),
            }
        }
        self.cfgs = flattened;
    }

    /// Gets this sequence as a slice.
    pub fn as_slice(&self) -> &[Cfg] {
        &self.cfgs
    }

    /// Gets this sequence as a mutable slice.
    pub fn as_slice_mut(&mut self) -> &mut [Cfg] {
        &mut self.cfgs
    }

    /// Converts this to a `Cfg`, unwrapping it when it is a sinlgleton.
    pub fn into_cfg(mut self) -> Cfg {
        if self.cfgs.len() == 1 {
            self.cfgs.pop().unwrap()
        } else {
            Cfg::Seq(self)
        }
    }
}

impl Debug for Seq {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Seq ")?;
        f.debug_list().entries(&self.cfgs).finish()
    }
}

impl Deref for Seq {
    type Target = [Cfg];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl DerefMut for Seq {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_slice_mut()
    }
}
