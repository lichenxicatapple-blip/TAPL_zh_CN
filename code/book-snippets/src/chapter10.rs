//! Rust counterparts for the OCaml fragments in Chapter 10.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Info {
    pub line: usize,
    pub column: usize,
}

pub mod initial_context {
    // TAPL-SNIPPET-BEGIN: ch10-context
    pub type Context = Vec<(String, Binding)>;
    // TAPL-SNIPPET-END: ch10-context

    // TAPL-SNIPPET-BEGIN: ch10-binding-name
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Binding {
        Name,
    }
    // TAPL-SNIPPET-END: ch10-binding-name
}

// TAPL-SNIPPET-BEGIN: ch10-type
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Bool,
    Arrow(Box<Type>, Box<Type>),
}
// TAPL-SNIPPET-END: ch10-type

// TAPL-SNIPPET-BEGIN: ch10-binding-var
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Binding {
    Name,
    Var(Type),
}
// TAPL-SNIPPET-END: ch10-binding-var

pub type Context = Vec<(String, Binding)>;

// TAPL-SNIPPET-BEGIN: ch10-add-binding
pub fn add_binding(mut context: Context, name: String, binding: Binding) -> Context {
    context.insert(0, (name, binding));
    context
}
// TAPL-SNIPPET-END: ch10-add-binding

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeError {
    BadIndex { info: Info, index: usize },
    WrongBinding { info: Info, index: usize },
    ExpectedBoolean { info: Info },
    BranchMismatch { info: Info },
    ExpectedArrow { info: Info },
    ParameterMismatch { info: Info },
}

// TAPL-SNIPPET-BEGIN: ch10-get-binding
pub fn get_binding(info: Info, context: &Context, index: usize) -> Result<&Binding, TypeError> {
    context
        .get(index)
        .map(|(_, binding)| binding)
        .ok_or(TypeError::BadIndex { info, index })
}
// TAPL-SNIPPET-END: ch10-get-binding

// TAPL-SNIPPET-BEGIN: ch10-get-type-from-context
pub fn get_type_from_context(
    info: Info,
    context: &Context,
    index: usize,
) -> Result<Type, TypeError> {
    match get_binding(info, context, index)? {
        Binding::Var(term_type) => Ok(term_type.clone()),
        Binding::Name => Err(TypeError::WrongBinding { info, index }),
    }
}
// TAPL-SNIPPET-END: ch10-get-type-from-context

// TAPL-SNIPPET-BEGIN: ch10-error
pub fn error(info: Info, message: &str) -> ! {
    panic!("{}:{}: {message}", info.line, info.column)
}
// TAPL-SNIPPET-END: ch10-error

// TAPL-SNIPPET-BEGIN: ch10-term
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    True(Info),
    False(Info),
    If(Info, Box<Term>, Box<Term>, Box<Term>),
    Var(Info, usize, usize),
    Abs(Info, String, Type, Box<Term>),
    App(Info, Box<Term>, Box<Term>),
}
// TAPL-SNIPPET-END: ch10-term

// TAPL-SNIPPET-BEGIN: ch10-type-of
pub fn type_of(context: &Context, term: &Term) -> Result<Type, TypeError> {
    match term {
        Term::True(_) | Term::False(_) => Ok(Type::Bool),
        Term::If(info, guard, then_term, else_term) => {
            if type_of(context, guard)? != Type::Bool {
                return Err(TypeError::ExpectedBoolean { info: *info });
            }
            let then_type = type_of(context, then_term)?;
            if then_type == type_of(context, else_term)? {
                Ok(then_type)
            } else {
                Err(TypeError::BranchMismatch { info: *info })
            }
        }
        Term::Var(info, index, _) => get_type_from_context(*info, context, *index),
        Term::Abs(_, name, parameter_type, body) => {
            let extended = add_binding(
                context.clone(),
                name.clone(),
                Binding::Var(parameter_type.clone()),
            );
            Ok(Type::Arrow(
                Box::new(parameter_type.clone()),
                Box::new(type_of(&extended, body)?),
            ))
        }
        Term::App(info, function, argument) => {
            let function_type = type_of(context, function)?;
            let argument_type = type_of(context, argument)?;
            match function_type {
                Type::Arrow(parameter_type, result_type) => {
                    if argument_type == *parameter_type {
                        Ok(*result_type)
                    } else {
                        Err(TypeError::ParameterMismatch { info: *info })
                    }
                }
                Type::Bool => Err(TypeError::ExpectedArrow { info: *info }),
            }
        }
    }
}
// TAPL-SNIPPET-END: ch10-type-of

// TAPL-SNIPPET-BEGIN: ch10-structural-equality
pub fn structurally_equal(function: Term, argument: Term) -> bool {
    let first = Term::App(
        Info::default(),
        Box::new(function.clone()),
        Box::new(argument.clone()),
    );
    let second = Term::App(Info::default(), Box::new(function), Box::new(argument));
    first == second
}
// TAPL-SNIPPET-END: ch10-structural-equality

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_has_an_arrow_type() {
        let identity = Term::Abs(
            Info::default(),
            "x".into(),
            Type::Bool,
            Box::new(Term::Var(Info::default(), 0, 1)),
        );
        assert_eq!(
            type_of(&Vec::new(), &identity),
            Ok(Type::Arrow(Box::new(Type::Bool), Box::new(Type::Bool)))
        );
    }

    #[test]
    fn derived_equality_is_structural() {
        assert!(structurally_equal(
            Term::True(Info::default()),
            Term::False(Info::default())
        ));
    }
}
