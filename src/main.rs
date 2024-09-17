use std::{env, fs, process};

use bfrs::{graph::Graph, ir::Ir, Ast};

fn main() {
    let args = env::args_os();
    if args.len() != 2 {
        eprintln!("Usage: bfrs <PROGRAM>");
        process::exit(2);
    }
    let path = args.skip(1).next().unwrap();
    let src = match fs::read(&path) {
        Ok(src) => src,
        Err(err) => {
            eprintln!("read {}: {err}", path.to_string_lossy());
            process::exit(2);
        }
    };
    let Some(ast) = Ast::parse(&src) else {
        eprintln!("parse error");
        process::exit(1);
    };
    let mut g = Graph::new();
    let mut ir = Ir::lower(&ast, &mut g);
    ir.optimize(&mut g);
    print!("{}", ir.pretty(&g));
}
