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
}
