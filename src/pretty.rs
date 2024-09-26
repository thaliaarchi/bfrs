use std::fmt::{self, Display, Formatter, Write};

use crate::{
    graph::{hash_arena::ArenaRef, Graph, NodeId},
    node::{Condition, Node},
    region::{Effect, Region},
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

    fn pretty_blocks(&mut self, blocks: &[NodeId], g: &Graph, indent: usize) -> fmt::Result {
        if let &[block] = blocks {
            let block = g.get(block);
            if let Node::BasicBlock(bb) = &*block {
                self.pretty_region(bb, g, indent)
            } else {
                self.pretty_node(&block, indent)
            }
        } else {
            for &block in blocks {
                let block = g.get(block);
                if let Node::BasicBlock(bb) = &*block {
                    self.indent(indent)?;
                    write!(self.w, "{{\n")?;
                    self.pretty_region(bb, g, indent + 1)?;
                    self.indent(indent)?;
                    write!(self.w, "}}\n")?;
                } else {
                    self.pretty_node(&block, indent)?;
                }
            }
            Ok(())
        }
    }

    fn pretty_node(&mut self, node: &ArenaRef<'_, Node>, indent: usize) -> fmt::Result {
        fn group(f: &mut dyn Write, grouped: bool, node: ArenaRef<'_, Node>) -> fmt::Result {
            if grouped {
                write!(f, "({node})")
            } else {
                write!(f, "{node}")
            }
        }
        let g = node.graph();
        match node.value() {
            Node::Root { blocks } => self.pretty_blocks(blocks, g, indent),
            Node::BasicBlock(bb) => self.pretty_region(bb, g, indent),
            Node::Loop { condition, body } => {
                self.indent(indent)?;
                match *condition {
                    Condition::WhileNonZero => write!(self.w, "while @0 != 0")?,
                    Condition::IfNonZero => write!(self.w, "if @0 != 0")?,
                    Condition::Count(count) => write!(self.w, "repeat {} times", g.get(count))?,
                }
                write!(self.w, " {{\n")?;
                self.pretty_blocks(body, g, indent + 1)?;
                self.indent(indent)?;
                write!(self.w, "}}\n")
            }
            &Node::Copy(offset) => write!(self.w, "@{offset}"),
            &Node::Const(value) => write!(self.w, "{}", value as i8),
            &Node::Input { id } => write!(self.w, "in{id}"),
            &Node::Add(lhs, rhs) => {
                let (lhs, rhs) = (g.get(lhs), g.get(rhs));
                write!(self.w, "{}", lhs)?;
                if let Node::Const(rhs) = *rhs {
                    if (rhs as i8) < 0 {
                        return write!(self.w, " - {}", (rhs as i8).unsigned_abs());
                    }
                }
                write!(self.w, " + ")?;
                group(self.w, matches!(*rhs, Node::Add(..)), rhs)
            }
            &Node::Mul(lhs, rhs) => {
                let (lhs, rhs) = (g.get(lhs), g.get(rhs));
                group(self.w, matches!(*lhs, Node::Add(..)), lhs)?;
                write!(self.w, " * ")?;
                group(self.w, matches!(*rhs, Node::Add(..) | Node::Mul(..)), rhs)
            }
            Node::Array(elements) => {
                fn write_char(w: &mut dyn Write, ch: u8) -> fmt::Result {
                    match ch {
                        b'\0' => write!(w, "\\0"),
                        b'\t' => write!(w, "\\t"),
                        b'\n' => write!(w, "\\n"),
                        b'\r' => write!(w, "\\r"),
                        b'\\' => write!(w, "\\\\"),
                        _ if ch.is_ascii() && !ch.is_ascii_control() => {
                            write!(w, "{}", ch as char)
                        }
                        _ => write!(w, "\\x{ch:02x}"),
                    }
                }
                if elements
                    .iter()
                    .all(|&e| matches!(*g.get(e), Node::Const(_)))
                {
                    write!(self.w, "\"")?;
                    for &e in elements {
                        let Node::Const(ch) = *g.get(e) else {
                            unreachable!();
                        };
                        write_char(self.w, ch)?;
                    }
                    write!(self.w, "\"")
                } else {
                    write!(self.w, "[")?;
                    for (i, &e) in elements.iter().enumerate() {
                        if i != 0 {
                            write!(self.w, ", ")?;
                        }
                        let e = g.get(e);
                        if let Node::Const(ch) = *e {
                            write!(self.w, "'")?;
                            write_char(self.w, ch)?;
                            write!(self.w, "'")?;
                        } else {
                            write!(self.w, "{e}")?;
                        }
                    }
                    write!(self.w, "]")
                }
            }
        }
    }

    fn pretty_region(&mut self, region: &Region, g: &Graph, indent: usize) -> fmt::Result {
        // Write effects and track discrepancies between them and the statistic
        // fields.
        let mut guarded_left = 0;
        let mut guarded_right = 0;
        let mut inputs = 0;
        for effect in &region.effects {
            self.indent(indent)?;
            match *effect {
                Effect::Output(value) => {
                    write!(self.w, "output {}", g.get(value))?;
                }
                Effect::Input(value) => {
                    let value = g.get(value);
                    write!(self.w, "{value} = input")?;
                    if let Node::Input { id } = *value {
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
        let memory = &region.memory;
        if memory.guarded_left() != guarded_left
            || memory.guarded_right() != guarded_right
            || region.inputs != inputs
        {
            self.indent(indent)?;
            write!(self.w, "# BUG:")?;
            if memory.guarded_left() != guarded_left {
                write!(self.w, " guarded_left={}", memory.guarded_left())?;
            }
            if memory.guarded_right() != guarded_right {
                write!(self.w, " guarded_right={}", memory.guarded_right())?;
            }
            if region.inputs != inputs {
                write!(self.w, " inputs={}", region.inputs)?;
            }
            write!(self.w, "\n")?;
        }

        for (offset, node) in memory.iter() {
            let node = g.get(node);
            if *node != Node::Copy(offset) {
                self.indent(indent)?;
                write!(self.w, "@{offset} = {node}\n")?;
            }
        }

        if memory.offset() != 0 {
            self.indent(indent)?;
            write!(self.w, "shift {}\n", memory.offset())?;
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

impl Display for ArenaRef<'_, Node> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        PrettyPrinter::new(f).pretty_node(self, 0)
    }
}

impl ArenaRef<'_, Node> {
    pub fn compare_pretty(&self, expect: &str) -> bool {
        let got = self.to_string();
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
