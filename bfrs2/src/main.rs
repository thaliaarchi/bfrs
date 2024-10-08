use std::{env::args_os, error::Error, fs, process::exit};

use bfrs2::{arena::Arena, optimize::unsound_outline_guards};

fn main() {
    if let Err(err) = do_main() {
        eprintln!("Error: {err}");
        exit(1);
    }
}

fn do_main() -> Result<(), Box<dyn Error>> {
    let args = args_os();
    if args.len() != 2 {
        eprintln!("Usage: bfrs-minimal PROGRAM");
        exit(2);
    }
    let filename = args.skip(1).next().unwrap();
    let src = fs::read(&filename)?;
    let mut a = Arena::new();
    let mut cfg = a.parse(&src)?;
    unsound_outline_guards(true);
    cfg.opt_closed_form_add(&mut a);
    cfg.opt_peel(&mut a);
    print!("{}", cfg.pretty(&a));
    Ok(())
}
