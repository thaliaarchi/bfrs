use std::fmt::{self, Display, Formatter};

/// Brainfuck abstract syntax tree.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Ast {
    /// `>`
    Right,
    /// `<`
    Left,
    /// `+`
    Inc,
    /// `-`
    Dec,
    /// `.`
    Output,
    /// `,`
    Input,
    /// `[`…`]`
    Loop(Vec<Ast>),
    /// Root of AST.
    Root(Vec<Ast>),
}

impl Ast {
    /// Parses a Brainfuck program to an AST.
    pub fn parse(src: &[u8]) -> Option<Self> {
        fn parse_block<I: Iterator<Item = u8>>(src: &mut I) -> Option<(Vec<Ast>, bool)> {
            let mut block = Vec::new();
            while let Some(ch) = src.next() {
                match ch {
                    b'>' => block.push(Ast::Right),
                    b'<' => block.push(Ast::Left),
                    b'+' => block.push(Ast::Inc),
                    b'-' => block.push(Ast::Dec),
                    b'.' => block.push(Ast::Output),
                    b',' => block.push(Ast::Input),
                    b'[' => {
                        let Some((body, true)) = parse_block(src) else {
                            return None;
                        };
                        block.push(Ast::Loop(body))
                    }
                    b']' => return Some((block, true)),
                    _ => {}
                }
            }
            Some((block, false))
        }

        if let Some((root, false)) = parse_block(&mut src.iter().copied()) {
            Some(Ast::Root(root))
        } else {
            None
        }
    }
}

impl Display for Ast {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Ast::Right => write!(f, ">"),
            Ast::Left => write!(f, "<"),
            Ast::Inc => write!(f, "+"),
            Ast::Dec => write!(f, "-"),
            Ast::Output => write!(f, "."),
            Ast::Input => write!(f, ","),
            Ast::Loop(body) => {
                write!(f, "[")?;
                for node in body {
                    write!(f, "{node}")?;
                }
                write!(f, "]")
            }
            Ast::Root(body) => {
                for node in body {
                    write!(f, "{node}")?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Ast;

    #[test]
    fn parse() {
        assert_eq!(
            Ast::parse(b"+[+[[[-][+]-[+]]][-]]+"),
            Some(Ast::Root(vec![
                Ast::Inc,
                Ast::Loop(vec![
                    Ast::Inc,
                    Ast::Loop(vec![Ast::Loop(vec![
                        Ast::Loop(vec![Ast::Dec]),
                        Ast::Loop(vec![Ast::Inc]),
                        Ast::Dec,
                        Ast::Loop(vec![Ast::Inc]),
                    ])]),
                    Ast::Loop(vec![Ast::Dec]),
                ]),
                Ast::Inc,
            ])),
        );
        assert_eq!(Ast::parse(b"["), None);
        assert_eq!(Ast::parse(b"[["), None);
        assert_eq!(Ast::parse(b"]"), None);
    }
}
