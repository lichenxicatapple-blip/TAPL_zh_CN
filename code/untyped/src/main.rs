use std::{env, fs, io};

use tapl_untyped::{Statement, eval, parse_program, print_term};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = if let Some(path) = env::args_os().nth(1) {
        fs::read_to_string(path)?
    } else {
        io::read_to_string(io::stdin())?
    };

    let mut context = Vec::new();
    for statement in parse_program(&source)? {
        match statement {
            Statement::Bind(name) => context.insert(0, name),
            Statement::Eval(term) => {
                let value = eval(term)?;
                println!("{}", print_term(&value, &context)?);
            }
        }
    }
    Ok(())
}
