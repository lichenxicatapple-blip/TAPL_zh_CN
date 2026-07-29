use std::{env, fs, io};

use tapl_arith::{eval, parse_program};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = if let Some(path) = env::args_os().nth(1) {
        fs::read_to_string(path)?
    } else {
        io::read_to_string(io::stdin())?
    };

    for term in parse_program(&source)? {
        println!("{}", eval(term));
    }
    Ok(())
}
