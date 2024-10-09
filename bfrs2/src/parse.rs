use std::{
    error::Error,
    fmt::{self, Display, Formatter},
    slice::Iter,
};

use crate::{
    block::BlockBuilder,
    cfg::{Cfg, Seq},
    egraph::Graph,
};

/// An error from parsing a Brainfuck program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// Unmatched `[`.
    UnclosedLoop,
    /// Unmatched `]`.
    UnopenedLoop,
}

impl Graph {
    /// Parses a Brainfuck program to a CFG.
    pub fn parse(&mut self, src: &[u8]) -> Result<Cfg, ParseError> {
        Parser::new(src, self).parse(true)
    }
}

struct Parser<'s, 'g> {
    src: Iter<'s, u8>,
    g: &'g mut Graph,
}

impl<'s, 'g> Parser<'s, 'g> {
    /// Constructs a new parser.
    fn new(src: &'s [u8], g: &'g mut Graph) -> Self {
        Parser { src: src.iter(), g }
    }

    /// Parses the root or a loop.
    fn parse(&mut self, root: bool) -> Result<Cfg, ParseError> {
        let mut seq = Vec::new();
        let mut block = BlockBuilder::new(self.g);
        let mut loop_closed = root;
        while let Some(ch) = self.src.next() {
            match ch {
                b'>' => block.shift(1),
                b'<' => block.shift(-1),
                b'+' => block.add(1),
                b'-' => block.add(255),
                b'.' => block.output(self.g),
                b',' => block.input(self.g),
                b'[' => {
                    if !block.is_empty() {
                        seq.push(Cfg::Block(block.finish(self.g)));
                    }
                    seq.push(Cfg::Loop(Box::new(self.parse(false)?)));
                }
                b']' => {
                    if root {
                        return Err(ParseError::UnopenedLoop);
                    }
                    loop_closed = true;
                    break;
                }
                _ => {}
            }
        }
        if !loop_closed {
            return Err(ParseError::UnclosedLoop);
        }
        if !block.is_empty() {
            if seq.is_empty() {
                return Ok(Cfg::Block(block.finish(self.g)));
            }
            seq.push(Cfg::Block(block.finish(self.g)));
        }
        Ok(Seq::from_unflattened(seq).into_cfg())
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let msg = match self {
            ParseError::UnclosedLoop => "unmatched [",
            ParseError::UnopenedLoop => "unmatched ]",
        };
        f.write_str(msg)
    }
}

impl Error for ParseError {}
