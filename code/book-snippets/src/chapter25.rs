//! Chapter 25: nameless System F with existential packages.

// TAPL-SNIPPET-BEGIN: ch25-type-definition
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Var(usize, usize),
    Arrow(Box<Type>, Box<Type>),
    All(String, Box<Type>),
    Some(String, Box<Type>),
}
// TAPL-SNIPPET-END: ch25-type-definition

// TAPL-SNIPPET-BEGIN: ch25-term-definition
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    Var(usize, usize),
    Abs(String, Type, Box<Term>),
    App(Box<Term>, Box<Term>),
    TypeAbs(String, Box<Term>),
    TypeApp(Box<Term>, Type),
    Pack(Type, Box<Term>, Type),
    Unpack(String, String, Box<Term>, Box<Term>),
}
// TAPL-SNIPPET-END: ch25-term-definition

// TAPL-SNIPPET-BEGIN: ch25-binding-error-support
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Binding {
    Name,
    Variable(Type),
    TypeVariable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    InvalidShift,
    UnboundVariable(usize),
    ExpectedArrow(Type),
    ExpectedUniversal(Type),
    ExpectedExistential(Type),
    ParameterMismatch { expected: Type, actual: Type },
    PackageMismatch { expected: Type, actual: Type },
    NoRuleApplies,
}

fn shifted_index(index: usize, distance: isize) -> Result<usize, Error> {
    index
        .checked_add_signed(distance)
        .ok_or(Error::InvalidShift)
}
// TAPL-SNIPPET-END: ch25-binding-error-support

// TAPL-SNIPPET-BEGIN: ch25-type-shift
/// Shifts every free type variable by 'distance', leaving variables below
/// 'cutoff' untouched. The stored context length is shifted for every leaf.
pub fn type_shift_above(distance: isize, cutoff: usize, ty: &Type) -> Result<Type, Error> {
    fn walk(distance: isize, cutoff: usize, ty: &Type) -> Result<Type, Error> {
        Ok(match ty {
            Type::Var(index, context_len) => Type::Var(
                if *index >= cutoff {
                    shifted_index(*index, distance)?
                } else {
                    *index
                },
                shifted_index(*context_len, distance)?,
            ),
            Type::Arrow(parameter, result) => Type::Arrow(
                Box::new(walk(distance, cutoff, parameter)?),
                Box::new(walk(distance, cutoff, result)?),
            ),
            Type::All(name, body) => {
                Type::All(name.clone(), Box::new(walk(distance, cutoff + 1, body)?))
            }
            Type::Some(name, body) => {
                Type::Some(name.clone(), Box::new(walk(distance, cutoff + 1, body)?))
            }
        })
    }
    walk(distance, cutoff, ty)
}

pub fn type_shift(distance: isize, ty: &Type) -> Result<Type, Error> {
    type_shift_above(distance, 0, ty)
}
// TAPL-SNIPPET-END: ch25-type-shift

// TAPL-SNIPPET-BEGIN: ch25-type-substitution
/// Replaces type variable 'variable' by 'replacement'. When traversal passes
/// under a quantifier, the target and replacement are lifted together.
pub fn type_substitute(replacement: &Type, variable: usize, ty: &Type) -> Result<Type, Error> {
    fn walk(replacement: &Type, variable: usize, cutoff: usize, ty: &Type) -> Result<Type, Error> {
        Ok(match ty {
            Type::Var(index, _) if *index == variable + cutoff => type_shift(
                isize::try_from(cutoff).map_err(|_| Error::InvalidShift)?,
                replacement,
            )?,
            Type::Var(index, context_len) => Type::Var(*index, *context_len),
            Type::Arrow(parameter, result) => Type::Arrow(
                Box::new(walk(replacement, variable, cutoff, parameter)?),
                Box::new(walk(replacement, variable, cutoff, result)?),
            ),
            Type::All(name, body) => Type::All(
                name.clone(),
                Box::new(walk(replacement, variable, cutoff + 1, body)?),
            ),
            Type::Some(name, body) => Type::Some(
                name.clone(),
                Box::new(walk(replacement, variable, cutoff + 1, body)?),
            ),
        })
    }
    walk(replacement, variable, 0, ty)
}

/// Opens one type binder: lift the replacement, substitute variable zero,
/// then remove the binder from all remaining indices.
pub fn type_substitute_top(replacement: &Type, body: &Type) -> Result<Type, Error> {
    type_shift(-1, &type_substitute(&type_shift(1, replacement)?, 0, body)?)
}
// TAPL-SNIPPET-END: ch25-type-substitution

// TAPL-SNIPPET-BEGIN: ch25-term-operations
/// Shifts both term variables and type variables embedded in annotations,
/// because the implementation stores both kinds of binding in one context.
pub fn term_shift_above(distance: isize, cutoff: usize, term: &Term) -> Result<Term, Error> {
    fn walk(distance: isize, cutoff: usize, term: &Term) -> Result<Term, Error> {
        Ok(match term {
            Term::Var(index, context_len) => Term::Var(
                if *index >= cutoff {
                    shifted_index(*index, distance)?
                } else {
                    *index
                },
                shifted_index(*context_len, distance)?,
            ),
            Term::Abs(name, parameter, body) => Term::Abs(
                name.clone(),
                type_shift_above(distance, cutoff, parameter)?,
                Box::new(walk(distance, cutoff + 1, body)?),
            ),
            Term::App(function, argument) => Term::App(
                Box::new(walk(distance, cutoff, function)?),
                Box::new(walk(distance, cutoff, argument)?),
            ),
            Term::TypeAbs(name, body) => {
                Term::TypeAbs(name.clone(), Box::new(walk(distance, cutoff + 1, body)?))
            }
            Term::TypeApp(function, argument) => Term::TypeApp(
                Box::new(walk(distance, cutoff, function)?),
                type_shift_above(distance, cutoff, argument)?,
            ),
            Term::Pack(hidden, value, package) => Term::Pack(
                type_shift_above(distance, cutoff, hidden)?,
                Box::new(walk(distance, cutoff, value)?),
                type_shift_above(distance, cutoff, package)?,
            ),
            Term::Unpack(type_name, name, package, body) => Term::Unpack(
                type_name.clone(),
                name.clone(),
                Box::new(walk(distance, cutoff, package)?),
                Box::new(walk(distance, cutoff + 2, body)?),
            ),
        })
    }
    walk(distance, cutoff, term)
}

pub fn term_shift(distance: isize, term: &Term) -> Result<Term, Error> {
    term_shift_above(distance, 0, term)
}

/// Replaces one free term variable while lifting the replacement whenever the
/// traversal enters a term or type binder, so no free variable is captured.
pub fn term_substitute(replacement: &Term, variable: usize, term: &Term) -> Result<Term, Error> {
    fn walk(
        replacement: &Term,
        variable: usize,
        cutoff: usize,
        term: &Term,
    ) -> Result<Term, Error> {
        Ok(match term {
            Term::Var(index, _) if *index == variable + cutoff => term_shift(
                isize::try_from(cutoff).map_err(|_| Error::InvalidShift)?,
                replacement,
            )?,
            Term::Var(index, context_len) => Term::Var(*index, *context_len),
            Term::Abs(name, parameter, body) => Term::Abs(
                name.clone(),
                parameter.clone(),
                Box::new(walk(replacement, variable, cutoff + 1, body)?),
            ),
            Term::App(function, argument) => Term::App(
                Box::new(walk(replacement, variable, cutoff, function)?),
                Box::new(walk(replacement, variable, cutoff, argument)?),
            ),
            Term::TypeAbs(name, body) => Term::TypeAbs(
                name.clone(),
                Box::new(walk(replacement, variable, cutoff + 1, body)?),
            ),
            Term::TypeApp(function, argument) => Term::TypeApp(
                Box::new(walk(replacement, variable, cutoff, function)?),
                argument.clone(),
            ),
            Term::Pack(hidden, value, package) => Term::Pack(
                hidden.clone(),
                Box::new(walk(replacement, variable, cutoff, value)?),
                package.clone(),
            ),
            Term::Unpack(type_name, name, package, body) => Term::Unpack(
                type_name.clone(),
                name.clone(),
                Box::new(walk(replacement, variable, cutoff, package)?),
                Box::new(walk(replacement, variable, cutoff + 2, body)?),
            ),
        })
    }
    walk(replacement, variable, 0, term)
}

pub fn term_substitute_top(replacement: &Term, body: &Term) -> Result<Term, Error> {
    term_shift(-1, &term_substitute(&term_shift(1, replacement)?, 0, body)?)
}

/// Replaces a type variable throughout every type annotation embedded in a
/// term. Term variables are left unchanged; type binders increase the cutoff.
pub fn type_term_substitute(
    replacement: &Type,
    variable: usize,
    term: &Term,
) -> Result<Term, Error> {
    fn walk(
        replacement: &Type,
        variable: usize,
        cutoff: usize,
        term: &Term,
    ) -> Result<Term, Error> {
        Ok(match term {
            Term::Var(index, context_len) => Term::Var(*index, *context_len),
            Term::Abs(name, parameter, body) => Term::Abs(
                name.clone(),
                type_substitute(replacement, variable + cutoff, parameter)?,
                Box::new(walk(replacement, variable, cutoff + 1, body)?),
            ),
            Term::App(function, argument) => Term::App(
                Box::new(walk(replacement, variable, cutoff, function)?),
                Box::new(walk(replacement, variable, cutoff, argument)?),
            ),
            Term::TypeAbs(name, body) => Term::TypeAbs(
                name.clone(),
                Box::new(walk(replacement, variable, cutoff + 1, body)?),
            ),
            Term::TypeApp(function, argument) => Term::TypeApp(
                Box::new(walk(replacement, variable, cutoff, function)?),
                type_substitute(replacement, variable + cutoff, argument)?,
            ),
            Term::Pack(hidden, value, package) => Term::Pack(
                type_substitute(replacement, variable + cutoff, hidden)?,
                Box::new(walk(replacement, variable, cutoff, value)?),
                type_substitute(replacement, variable + cutoff, package)?,
            ),
            Term::Unpack(type_name, name, package, body) => Term::Unpack(
                type_name.clone(),
                name.clone(),
                Box::new(walk(replacement, variable, cutoff, package)?),
                Box::new(walk(replacement, variable, cutoff + 2, body)?),
            ),
        })
    }
    walk(replacement, variable, 0, term)
}

pub fn type_term_substitute_top(replacement: &Type, body: &Term) -> Result<Term, Error> {
    term_shift(
        -1,
        &type_term_substitute(&type_shift(1, replacement)?, 0, body)?,
    )
}
// TAPL-SNIPPET-END: ch25-term-operations

// TAPL-SNIPPET-BEGIN: ch25-evaluation-support
fn is_value(term: &Term) -> bool {
    match term {
        Term::Abs(..) | Term::TypeAbs(..) => true,
        Term::Pack(_, value, _) => is_value(value),
        _ => false,
    }
}
// TAPL-SNIPPET-END: ch25-evaluation-support

// TAPL-SNIPPET-BEGIN: ch25-evaluation
/// Performs one call-by-value evaluation step for the System F constructs.
pub fn evaluate_one(term: &Term) -> Result<Term, Error> {
    Ok(match term {
        Term::App(function, argument) if is_value(function) && is_value(argument) => {
            if let Term::Abs(_, _, body) = function.as_ref() {
                term_substitute_top(argument, body)?
            } else {
                return Err(Error::NoRuleApplies);
            }
        }
        Term::App(function, argument) if is_value(function) => {
            Term::App(function.clone(), Box::new(evaluate_one(argument)?))
        }
        Term::App(function, argument) => {
            Term::App(Box::new(evaluate_one(function)?), argument.clone())
        }
        Term::TypeApp(function, argument) => {
            if let Term::TypeAbs(_, body) = function.as_ref() {
                type_term_substitute_top(argument, body)?
            } else {
                Term::TypeApp(Box::new(evaluate_one(function)?), argument.clone())
            }
        }
        Term::Unpack(type_name, name, package, body) => {
            if let Term::Pack(hidden, value, _) = package.as_ref() {
                if is_value(value) {
                    // The value originally lives outside the two binders X,x.
                    // Lift it once before substituting for x; opening X then
                    // removes the remaining type-binder slot.
                    let with_value = term_substitute_top(&term_shift(1, value)?, body)?;
                    type_term_substitute_top(hidden, &with_value)?
                } else {
                    Term::Unpack(
                        type_name.clone(),
                        name.clone(),
                        Box::new(evaluate_one(package)?),
                        body.clone(),
                    )
                }
            } else {
                Term::Unpack(
                    type_name.clone(),
                    name.clone(),
                    Box::new(evaluate_one(package)?),
                    body.clone(),
                )
            }
        }
        Term::Pack(hidden, value, package) => Term::Pack(
            hidden.clone(),
            Box::new(evaluate_one(value)?),
            package.clone(),
        ),
        _ => return Err(Error::NoRuleApplies),
    })
}
// TAPL-SNIPPET-END: ch25-evaluation

// TAPL-SNIPPET-BEGIN: ch25-typechecking-support
fn binding_type(context: &[Binding], index: usize) -> Result<Type, Error> {
    let binding = context
        .iter()
        .rev()
        .nth(index)
        .ok_or(Error::UnboundVariable(index))?;
    match binding {
        Binding::Variable(ty) => type_shift(
            isize::try_from(index + 1).map_err(|_| Error::InvalidShift)?,
            ty,
        ),
        _ => Err(Error::UnboundVariable(index)),
    }
}
// TAPL-SNIPPET-END: ch25-typechecking-support

// TAPL-SNIPPET-BEGIN: ch25-typechecking
/// Implements all typing rules for the chapter's System F constructs.
/// The final shift in T-Unpack rejects a result type containing hidden X.
pub fn type_of(context: &[Binding], term: &Term) -> Result<Type, Error> {
    match term {
        Term::Var(index, _) => binding_type(context, *index),
        Term::Abs(_, parameter, body) => {
            let mut extended = context.to_vec();
            extended.push(Binding::Variable(parameter.clone()));
            Ok(Type::Arrow(
                Box::new(parameter.clone()),
                Box::new(type_shift(-1, &type_of(&extended, body)?)?),
            ))
        }
        Term::App(function, argument) => {
            let function_type = type_of(context, function)?;
            let argument_type = type_of(context, argument)?;
            match function_type {
                Type::Arrow(parameter, result) if *parameter == argument_type => Ok(*result),
                Type::Arrow(parameter, _) => Err(Error::ParameterMismatch {
                    expected: *parameter,
                    actual: argument_type,
                }),
                other => Err(Error::ExpectedArrow(other)),
            }
        }
        Term::TypeAbs(name, body) => {
            let mut extended = context.to_vec();
            extended.push(Binding::TypeVariable);
            Ok(Type::All(name.clone(), Box::new(type_of(&extended, body)?)))
        }
        Term::TypeApp(function, argument) => match type_of(context, function)? {
            Type::All(_, body) => type_substitute_top(argument, &body),
            other => Err(Error::ExpectedUniversal(other)),
        },
        Term::Pack(hidden, value, package) => match package {
            Type::Some(_, body) => {
                let expected = type_substitute_top(hidden, body)?;
                let actual = type_of(context, value)?;
                if expected == actual {
                    Ok(package.clone())
                } else {
                    Err(Error::PackageMismatch { expected, actual })
                }
            }
            other => Err(Error::ExpectedExistential(other.clone())),
        },
        Term::Unpack(_, _, package, body) => match type_of(context, package)? {
            Type::Some(_, packed_body) => {
                let mut extended = context.to_vec();
                extended.push(Binding::TypeVariable);
                extended.push(Binding::Variable(*packed_body));
                type_shift(-2, &type_of(&extended, body)?)
            }
            other => Err(Error::ExpectedExistential(other)),
        },
    }
}
// TAPL-SNIPPET-END: ch25-typechecking

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polymorphic_identity_instantiates() {
        let identity = Term::TypeAbs(
            "X".into(),
            Box::new(Term::Abs(
                "x".into(),
                Type::Var(0, 1),
                Box::new(Term::Var(0, 2)),
            )),
        );
        let instantiated = Term::TypeApp(
            Box::new(identity),
            Type::All("Y".into(), Box::new(Type::Var(0, 1))),
        );
        assert!(matches!(type_of(&[], &instantiated), Ok(Type::Arrow(_, _))));
    }

    #[test]
    fn type_substitution_avoids_capture() {
        let body = Type::All(
            "Y".into(),
            Box::new(Type::Arrow(
                Box::new(Type::Var(1, 2)),
                Box::new(Type::Var(0, 2)),
            )),
        );
        let replacement = Type::All("Z".into(), Box::new(Type::Var(0, 1)));
        let result = type_substitute(&replacement, 0, &body).unwrap();
        assert!(matches!(result, Type::All(_, _)));
    }

    #[test]
    fn package_has_its_declared_existential_type() {
        let package_type = Type::Some(
            "X".into(),
            Box::new(Type::Arrow(
                Box::new(Type::Var(0, 1)),
                Box::new(Type::Var(0, 1)),
            )),
        );
        let hidden = Type::All("Y".into(), Box::new(Type::Var(0, 1)));
        let value = Term::Abs("n".into(), hidden.clone(), Box::new(Term::Var(0, 1)));
        let package = Term::Pack(hidden, Box::new(value), package_type);
        assert!(matches!(type_of(&[], &package), Ok(Type::Some(_, _))));
    }

    fn polymorphic_identity() -> Term {
        Term::TypeAbs(
            "X".into(),
            Box::new(Term::Abs(
                "x".into(),
                Type::Var(0, 1),
                Box::new(Term::Var(0, 2)),
            )),
        )
    }

    fn identity_type() -> Type {
        Type::All(
            "X".into(),
            Box::new(Type::Arrow(
                Box::new(Type::Var(0, 1)),
                Box::new(Type::Var(0, 1)),
            )),
        )
    }

    #[test]
    fn type_application_substitutes_in_term_annotations() {
        let instantiated = Term::TypeApp(Box::new(polymorphic_identity()), identity_type());
        let reduced = evaluate_one(&instantiated).unwrap();
        match reduced {
            Term::Abs(_, Type::All(_, _), body) => {
                assert_eq!(*body, Term::Var(0, 1));
            }
            other => panic!("unexpected reduced term: {other:?}"),
        }
    }

    #[test]
    fn unpack_substitutes_the_value_and_hidden_type() {
        let hidden = identity_type();
        let package_type = Type::Some("X".into(), Box::new(Type::Var(0, 1)));
        let package = Term::Pack(hidden, Box::new(polymorphic_identity()), package_type);
        let unpack = Term::Unpack(
            "X".into(),
            "x".into(),
            Box::new(package),
            Box::new(Term::Var(0, 2)),
        );
        assert_eq!(evaluate_one(&unpack).unwrap(), polymorphic_identity());
    }

    #[test]
    fn escaping_hidden_type_and_negative_shifts_are_rejected() {
        assert_eq!(type_shift(-1, &Type::Var(0, 1)), Err(Error::InvalidShift));

        let hidden = identity_type();
        let package = Term::Pack(
            hidden,
            Box::new(polymorphic_identity()),
            Type::Some("X".into(), Box::new(Type::Var(0, 1))),
        );
        let escaping = Term::Unpack(
            "X".into(),
            "x".into(),
            Box::new(package),
            Box::new(Term::Var(0, 2)),
        );
        assert_eq!(type_of(&[], &escaping), Err(Error::InvalidShift));
    }

    #[test]
    fn typechecker_reports_major_mismatches() {
        let identity = identity_type();
        let function = Term::Abs("f".into(), identity.clone(), Box::new(Term::Var(0, 1)));
        let wrong_argument = Term::Abs(
            "g".into(),
            Type::Arrow(Box::new(identity.clone()), Box::new(identity.clone())),
            Box::new(Term::Var(0, 1)),
        );
        assert!(matches!(
            type_of(
                &[],
                &Term::App(Box::new(function), Box::new(wrong_argument))
            ),
            Err(Error::ParameterMismatch { .. })
        ));
        assert!(matches!(
            type_of(
                &[],
                &Term::TypeApp(Box::new(polymorphic_identity()), identity)
            ),
            Ok(Type::Arrow(_, _))
        ));
        assert!(matches!(
            type_of(
                &[],
                &Term::TypeApp(
                    Box::new(Term::Abs(
                        "x".into(),
                        identity_type(),
                        Box::new(Term::Var(0, 1))
                    )),
                    identity_type()
                )
            ),
            Err(Error::ExpectedUniversal(_))
        ));
    }
}
