use crate::block::Block;

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
}
