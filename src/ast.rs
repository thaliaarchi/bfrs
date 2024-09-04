/// Brainfuck abstract syntax tree.
#[derive(Clone, Debug, PartialEq, Eq)]
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
}

impl Ast {
    /// Parses a Brainfuck program to an AST.
    pub fn parse(src: &[u8]) -> Option<Vec<Self>> {
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
            Some(root)
        } else {
            None
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
            Some(vec![
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
            ]),
        );
        assert_eq!(Ast::parse(b"["), None);
        assert_eq!(Ast::parse(b"[["), None);
        assert_eq!(Ast::parse(b"]"), None);
    }
}
