use std::{env::args_os, error::Error, fs, process::exit};

use bfrs_minimal::arena::Arena;

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
    let cfg = a.parse(&src)?;
    print!("{}", cfg.pretty(&a));
    Ok(())
}
