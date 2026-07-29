use tapl_simple::{Store, Term, TypeContext, eval, type_of};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let example = Term::IsZero(Box::new(Term::Pred(Box::new(Term::Succ(Box::new(
        Term::Nat(0),
    ))))));
    let inferred = type_of(&example, &TypeContext::new(), &Vec::new())?;
    let value = eval(&example, &mut Store::new())?;
    println!("{value} : {inferred}");
    Ok(())
}
