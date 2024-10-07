use std::{
    fmt::{self, Debug, Formatter},
    ops::Deref,
    slice::Iter,
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

    /// Iterates mutably over `Seq`, then concatenates adjacent basic blocks
    /// and flattens top-level sequences.
    pub fn for_each(&mut self, a: &mut Arena, mut each: impl FnMut(&mut Cfg, &mut Arena)) {
        for cfg in &mut self.cfgs {
            each(cfg, a);
        }
        self.flatten(a);
    }

    /// Concanates adjacent blocks and flattens top-level sequences.
    fn flatten(&mut self, a: &mut Arena) {
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

impl<'a> IntoIterator for &'a Seq {
    type Item = &'a Cfg;
    type IntoIter = Iter<'a, Cfg>;

    fn into_iter(self) -> Self::IntoIter {
        self.cfgs.iter()
    }
}

impl Deref for Seq {
    type Target = [Cfg];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}
