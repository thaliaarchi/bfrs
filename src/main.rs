use std::{env, fs, process};

use bfrs::{graph::Graph, Ast};

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
    let g = Graph::new();
    let ir = g.lower(&ast);
    g.optimize(g.get_mut(ir));
    print!("{}", g.get(ir).to_string());
}
