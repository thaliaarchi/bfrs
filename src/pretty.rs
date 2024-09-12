use std::fmt::{self, Write};

use crate::{
    graph::Graph,
    ir::{BasicBlock, Condition, Effect, Ir},
    node::Byte,
};

struct PrettyPrinter<'a> {
    w: &'a mut (dyn Write + 'a),
    indent_buf: String,
}

impl<'a> PrettyPrinter<'a> {
    const INDENT: &'static str = "    ";

    fn new(w: &'a mut (dyn Write + 'a)) -> Self {
        PrettyPrinter {
            w,
            indent_buf: Self::INDENT.repeat(4),
        }
    }

    fn pretty_blocks(&mut self, blocks: &[Ir], g: &Graph, indent: usize) -> fmt::Result {
        if let [Ir::BasicBlock(bb)] = blocks {
            self.pretty_bb(bb, g, indent)
        } else if let [block] = blocks {
            self.pretty_ir(block, g, indent)
        } else {
            for block in blocks {
                self.pretty_ir(block, g, indent)?;
            }
            Ok(())
        }
    }

    fn pretty_ir(&mut self, ir: &Ir, g: &Graph, indent: usize) -> fmt::Result {
        match ir {
            Ir::BasicBlock(bb) => {
                self.indent(indent)?;
                write!(self.w, "{{\n")?;
                self.pretty_bb(bb, g, indent + 1)?;
                self.indent(indent)?;
                write!(self.w, "}}\n")
            }
            Ir::Loop { condition, body } => {
                self.indent(indent)?;
                match condition {
                    Condition::WhileNonZero => write!(self.w, "while @0 != 0")?,
                    Condition::IfNonZero => write!(self.w, "if @0 != 0")?,
                    Condition::Count(count) => write!(self.w, "repeat {:?} times", count.get(g))?,
                }
                write!(self.w, " {{\n")?;
                self.pretty_blocks(body, g, indent + 1)?;
                self.indent(indent)?;
                write!(self.w, "}}\n")
            }
        }
    }

    fn pretty_bb(&mut self, bb: &BasicBlock, g: &Graph, indent: usize) -> fmt::Result {
        // Write effects and track discrepancies between them and the statistic
        // fields.
        let mut guarded_left = 0;
        let mut guarded_right = 0;
        let mut inputs = 0;
        for effect in &bb.effects {
            self.indent(indent)?;
            match *effect {
                Effect::Output(value) => write!(self.w, "output {:?}", value.get(g))?,
                Effect::Input(value) => {
                    write!(self.w, "{:?} = input", value.get(g))?;
                    if let Byte::Input { id } = g[value] {
                        if id != inputs {
                            write!(self.w, " # BUG: unordered")?;
                        }
                        inputs = id + 1;
                    } else {
                        write!(self.w, " # BUG: invalid type")?;
                    };
                }
                Effect::GuardShift(offset) => {
                    write!(self.w, "guard_shift {offset}")?;
                    if offset < 0 && offset < guarded_left {
                        guarded_left = offset;
                    } else if offset > 0 && offset > guarded_right {
                        guarded_right = offset;
                    } else if offset == 0 {
                        write!(self.w, " # BUG: zero")?;
                    } else {
                        write!(self.w, " # BUG: unordered")?;
                    }
                }
            }
            write!(self.w, "\n")?;
        }
        if bb.guarded_left != guarded_left
            || bb.guarded_right != guarded_right
            || bb.inputs != inputs
        {
            self.indent(indent)?;
            write!(self.w, "# BUG:")?;
            if bb.guarded_left != guarded_left {
                write!(self.w, " guarded_left={}", bb.guarded_left)?;
            }
            if bb.guarded_right != guarded_right {
                write!(self.w, " guarded_right={}", bb.guarded_right)?;
            }
            if bb.inputs != inputs {
                write!(self.w, " inputs={}", bb.inputs)?;
            }
            write!(self.w, "\n")?;
        }

        for (offset, node) in (bb.min_offset()..)
            .zip(bb.memory.iter())
            .filter(|&(offset, &node)| g[node] != Byte::Copy(offset))
        {
            self.indent(indent)?;
            write!(self.w, "@{offset} = {:?}\n", node.get(g))?;
        }

        if bb.offset != 0 {
            self.indent(indent)?;
            write!(self.w, "shift {}\n", bb.offset)?;
        }
        Ok(())
    }

    fn indent(&mut self, indent: usize) -> fmt::Result {
        let len = indent * Self::INDENT.len();
        self.indent_buf.reserve(len);
        while self.indent_buf.len() < len {
            self.indent_buf.push_str(Self::INDENT);
        }
        self.w.write_str(&self.indent_buf[..len])
    }
}

impl Ir {
    pub fn pretty(&self, g: &Graph) -> String {
        let mut s = String::new();
        self.write_pretty(&mut s, g).unwrap();
        s
    }

    pub fn write_pretty(&self, w: &mut dyn Write, g: &Graph) -> fmt::Result {
        PrettyPrinter::new(w).pretty_ir(self, g, 0)
    }

    pub fn compare_pretty(&self, expect: &str, g: &Graph) -> bool {
        compare_pretty(&self.pretty(g), expect)
    }

    pub fn pretty_root(blocks: &[Ir], g: &Graph) -> String {
        let mut s = String::new();
        Ir::write_pretty_root(blocks, &mut s, g).unwrap();
        s
    }

    pub fn write_pretty_root(blocks: &[Ir], w: &mut dyn Write, g: &Graph) -> fmt::Result {
        PrettyPrinter::new(w).pretty_blocks(blocks, g, 0)
    }

    pub fn compare_pretty_root(blocks: &[Ir], expect: &str, g: &Graph) -> bool {
        let mut s = String::new();
        Ir::write_pretty_root(blocks, &mut s, g).unwrap();
        compare_pretty(&s, expect)
    }
}

impl BasicBlock {
    pub fn pretty(&self, g: &Graph) -> String {
        let mut s = String::new();
        self.write_pretty(&mut s, g).unwrap();
        s
    }

    pub fn write_pretty(&self, w: &mut dyn Write, g: &Graph) -> fmt::Result {
        PrettyPrinter::new(w).pretty_bb(self, g, 0)
    }

    pub fn compare_pretty(&self, expect: &str, g: &Graph) -> bool {
        compare_pretty(&self.pretty(g), expect)
    }
}

fn compare_pretty(got: &str, expect: &str) -> bool {
    let expect = unindent(expect);
    if got == expect {
        true
    } else {
        println!("<<<<<<< Expect");
        print!("{expect}");
        println!("=======");
        print!("{got}");
        println!(">>>>>>> Got");
        false
    }
}

fn unindent(s: &str) -> String {
    fn get_indent(s: &str) -> &str {
        let len = s
            .as_bytes()
            .iter()
            .position(|&b| b != b' ')
            .unwrap_or(s.len());
        &s[..(len / PrettyPrinter::INDENT.len()) * PrettyPrinter::INDENT.len()]
    }
    let s = s
        .trim_start_matches(|c| c == '\n')
        .trim_end_matches(|c| c == ' ' || c == '\n');
    let mut lines = s.lines();
    let first_indent = get_indent(lines.next().unwrap_or_default());
    let mut min_indent = lines
        .map(|line| get_indent(line))
        .min_by(|&a, &b| a.len().cmp(&b.len()))
        .unwrap_or(first_indent);
    if !first_indent.is_empty() && first_indent.len() < min_indent.len() {
        min_indent = first_indent;
    }
    let mut unindented = String::with_capacity(s.len());
    for line in s.lines() {
        unindented.push_str(line.strip_prefix(min_indent).unwrap_or(line));
        unindented.push_str("\n");
    }
    unindented
}
