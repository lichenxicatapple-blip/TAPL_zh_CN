//! Type reconstruction for the simply typed lambda calculus (Chapter 22).

use std::collections::{BTreeMap, BTreeSet};

// TAPL-SNIPPET-BEGIN: ch22-types-generator
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Type {
    Bool,
    Nat,
    Variable(String),
    Arrow(Box<Type>, Box<Type>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    True,
    False,
    Zero,
    Successor(Box<Term>),
    Variable(String),
    Abstraction {
        parameter: String,
        annotation: Option<Type>,
        body: Box<Term>,
    },
    Application(Box<Term>, Box<Term>),
}

#[derive(Clone, Debug, Default)]
pub struct FreshVariables {
    next: usize,
}

impl FreshVariables {
    pub fn fresh(&mut self) -> Type {
        let name = format!("?X{}", self.next);
        self.next += 1;
        Type::Variable(name)
    }
}
// TAPL-SNIPPET-END: ch22-types-generator

pub type Substitution = BTreeMap<String, Type>;
pub type Constraints = Vec<(Type, Type)>;
pub type Context = BTreeMap<String, Type>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeError {
    UnknownVariable(String),
    CannotUnify(Type, Type),
    OccursCheck { variable: String, within: Type },
}

fn free_variables(ty: &Type, variables: &mut BTreeSet<String>) {
    match ty {
        Type::Variable(name) => {
            variables.insert(name.clone());
        }
        Type::Arrow(domain, codomain) => {
            free_variables(domain, variables);
            free_variables(codomain, variables);
        }
        Type::Bool | Type::Nat => {}
    }
}

fn apply(substitution: &Substitution, ty: &Type) -> Type {
    match ty {
        Type::Variable(name) => substitution.get(name).map_or_else(
            || ty.clone(),
            |replacement| apply(substitution, replacement),
        ),
        Type::Arrow(domain, codomain) => Type::Arrow(
            Box::new(apply(substitution, domain)),
            Box::new(apply(substitution, codomain)),
        ),
        Type::Bool | Type::Nat => ty.clone(),
    }
}

fn compose(newer: &Substitution, older: &Substitution) -> Substitution {
    let mut composed = older
        .iter()
        .map(|(name, ty)| (name.clone(), apply(newer, ty)))
        .collect::<Substitution>();
    composed.extend(newer.clone());
    composed
}

fn substitute_constraint(
    variable: &str,
    replacement: &Type,
    constraints: Constraints,
) -> Constraints {
    let one = Substitution::from([(variable.to_owned(), replacement.clone())]);
    constraints
        .into_iter()
        .map(|(left, right)| (apply(&one, &left), apply(&one, &right)))
        .collect()
}

// TAPL-SNIPPET-BEGIN: ch22-unify
/// Computes a most general unifier for a finite set of type equations.
///
/// The occurs check rejects cyclic equations such as `X = X -> Nat`, because
/// this chapter is reconstructing finite simple types.
pub fn unify(mut constraints: Constraints) -> Result<Substitution, TypeError> {
    let mut solution = Substitution::new();
    while let Some((left, right)) = constraints.pop() {
        let left = apply(&solution, &left);
        let right = apply(&solution, &right);
        if left == right {
            continue;
        }
        match (left, right) {
            (Type::Variable(variable), ty) | (ty, Type::Variable(variable)) => {
                let mut variables = BTreeSet::new();
                free_variables(&ty, &mut variables);
                if variables.contains(&variable) {
                    return Err(TypeError::OccursCheck {
                        variable,
                        within: ty,
                    });
                }
                constraints = substitute_constraint(&variable, &ty, constraints);
                let step = Substitution::from([(variable, ty)]);
                solution = compose(&step, &solution);
            }
            (Type::Arrow(s1, s2), Type::Arrow(t1, t2)) => {
                constraints.push((*s1, *t1));
                constraints.push((*s2, *t2));
            }
            (left, right) => return Err(TypeError::CannotUnify(left, right)),
        }
    }
    Ok(solution)
}
// TAPL-SNIPPET-END: ch22-unify

// TAPL-SNIPPET-BEGIN: ch22-constraints
/// Generates a result type and the equations that make a term typable.
pub fn generate_constraints(
    context: &Context,
    term: &Term,
    fresh: &mut FreshVariables,
) -> Result<(Type, Constraints), TypeError> {
    match term {
        Term::True | Term::False => Ok((Type::Bool, vec![])),
        Term::Zero => Ok((Type::Nat, vec![])),
        Term::Successor(argument) => {
            let (ty, mut constraints) = generate_constraints(context, argument, fresh)?;
            constraints.push((ty, Type::Nat));
            Ok((Type::Nat, constraints))
        }
        Term::Variable(name) => context
            .get(name)
            .cloned()
            .map(|ty| (ty, vec![]))
            .ok_or_else(|| TypeError::UnknownVariable(name.clone())),
        Term::Abstraction {
            parameter,
            annotation,
            body,
        } => {
            let parameter_type = annotation.clone().unwrap_or_else(|| fresh.fresh());
            let mut body_context = context.clone();
            body_context.insert(parameter.clone(), parameter_type.clone());
            let (body_type, constraints) = generate_constraints(&body_context, body, fresh)?;
            Ok((
                Type::Arrow(Box::new(parameter_type), Box::new(body_type)),
                constraints,
            ))
        }
        Term::Application(function, argument) => {
            let (function_type, mut constraints) = generate_constraints(context, function, fresh)?;
            let (argument_type, argument_constraints) =
                generate_constraints(context, argument, fresh)?;
            constraints.extend(argument_constraints);
            let result_type = fresh.fresh();
            constraints.push((
                function_type,
                Type::Arrow(Box::new(argument_type), Box::new(result_type.clone())),
            ));
            Ok((result_type, constraints))
        }
    }
}
// TAPL-SNIPPET-END: ch22-constraints

// TAPL-SNIPPET-BEGIN: ch22-principal-type
/// Reconstructs the principal type by generating and then unifying constraints.
pub fn principal_type(context: &Context, term: &Term) -> Result<Type, TypeError> {
    let mut fresh = FreshVariables::default();
    let (schematic_type, constraints) = generate_constraints(context, term, &mut fresh)?;
    let substitution = unify(constraints)?;
    Ok(apply(&substitution, &schematic_type))
}
// TAPL-SNIPPET-END: ch22-principal-type

#[cfg(test)]
mod tests {
    use super::*;

    fn variable(name: &str) -> Term {
        Term::Variable(name.to_owned())
    }

    #[test]
    fn reconstructs_identity() {
        let identity = Term::Abstraction {
            parameter: "x".into(),
            annotation: None,
            body: Box::new(variable("x")),
        };
        let ty = principal_type(&Context::new(), &identity).unwrap();
        assert!(matches!(ty, Type::Arrow(left, right) if left == right));
    }

    #[test]
    fn reconstructs_application() {
        let apply_to_zero = Term::Abstraction {
            parameter: "f".into(),
            annotation: None,
            body: Box::new(Term::Application(
                Box::new(variable("f")),
                Box::new(Term::Zero),
            )),
        };
        let ty = principal_type(&Context::new(), &apply_to_zero).unwrap();
        assert!(matches!(
            ty,
            Type::Arrow(domain, result)
                if matches!(*domain, Type::Arrow(ref argument, ref output)
                    if **argument == Type::Nat && **output == *result)
        ));
    }

    #[test]
    fn rejects_self_application_of_a_finite_type() {
        let self_application = Term::Abstraction {
            parameter: "x".into(),
            annotation: None,
            body: Box::new(Term::Application(
                Box::new(variable("x")),
                Box::new(variable("x")),
            )),
        };
        assert!(matches!(
            principal_type(&Context::new(), &self_application),
            Err(TypeError::OccursCheck { .. })
        ));
    }

    #[test]
    fn unifier_is_applied_transitively() {
        let substitution = unify(vec![
            (Type::Variable("X".into()), Type::Variable("Y".into())),
            (Type::Variable("Y".into()), Type::Nat),
        ])
        .unwrap();
        assert_eq!(apply(&substitution, &Type::Variable("X".into())), Type::Nat);
    }
}
