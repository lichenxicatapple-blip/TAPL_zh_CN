//! Type reconstruction for the simply typed lambda calculus (Chapter 22).

use std::collections::{BTreeMap, BTreeSet};

// TAPL-SNIPPET-BEGIN: ch22-types-generator
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Type {
    Bool,
    Nat,
    Unit,
    Variable(String),
    Arrow(Box<Type>, Box<Type>),
    Reference(Box<Type>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    True,
    False,
    Zero,
    Unit,
    Successor(Box<Term>),
    Predecessor(Box<Term>),
    IsZero(Box<Term>),
    If(Box<Term>, Box<Term>, Box<Term>),
    Variable(String),
    Abstraction {
        parameter: String,
        annotation: Option<Type>,
        body: Box<Term>,
    },
    Application(Box<Term>, Box<Term>),
    Reference(Box<Term>),
    Dereference(Box<Term>),
    Assignment(Box<Term>, Box<Term>),
    Sequence(Box<Term>, Box<Term>),
    Let {
        name: String,
        value: Box<Term>,
        body: Box<Term>,
    },
}

#[derive(Clone, Debug, Default)]
pub struct FreshVariables {
    next: usize,
    reserved: BTreeSet<String>,
}

impl FreshVariables {
    pub fn fresh(&mut self) -> Type {
        loop {
            let name = format!("?X{}", self.next);
            self.next += 1;
            if self.reserved.insert(name.clone()) {
                return Type::Variable(name);
            }
        }
    }

    fn reserve_type(&mut self, ty: &Type) {
        match ty {
            Type::Variable(name) => {
                self.reserved.insert(name.clone());
            }
            Type::Arrow(domain, codomain) => {
                self.reserve_type(domain);
                self.reserve_type(codomain);
            }
            Type::Reference(element) => self.reserve_type(element),
            Type::Bool | Type::Nat | Type::Unit => {}
        }
    }

    fn reserve_term_annotations(&mut self, term: &Term) {
        match term {
            Term::Abstraction {
                annotation, body, ..
            } => {
                if let Some(ty) = annotation {
                    self.reserve_type(ty);
                }
                self.reserve_term_annotations(body);
            }
            Term::Application(function, argument) => {
                self.reserve_term_annotations(function);
                self.reserve_term_annotations(argument);
            }
            Term::Let { value, body, .. } => {
                self.reserve_term_annotations(value);
                self.reserve_term_annotations(body);
            }
            Term::Successor(argument)
            | Term::Predecessor(argument)
            | Term::IsZero(argument)
            | Term::Reference(argument)
            | Term::Dereference(argument) => self.reserve_term_annotations(argument),
            Term::Assignment(target, value) | Term::Sequence(target, value) => {
                self.reserve_term_annotations(target);
                self.reserve_term_annotations(value);
            }
            Term::If(guard, then_term, else_term) => {
                self.reserve_term_annotations(guard);
                self.reserve_term_annotations(then_term);
                self.reserve_term_annotations(else_term);
            }
            Term::True | Term::False | Term::Zero | Term::Unit | Term::Variable(_) => {}
        }
    }
}
// TAPL-SNIPPET-END: ch22-types-generator

// TAPL-SNIPPET-BEGIN: ch22-inference-support
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
        Type::Reference(element) => free_variables(element, variables),
        Type::Bool | Type::Nat | Type::Unit => {}
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
        Type::Reference(element) => Type::Reference(Box::new(apply(substitution, element))),
        Type::Bool | Type::Nat | Type::Unit => ty.clone(),
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
// TAPL-SNIPPET-END: ch22-inference-support

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
            (Type::Reference(s), Type::Reference(t)) => constraints.push((*s, *t)),
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
        Term::Unit => Ok((Type::Unit, vec![])),
        Term::Successor(argument) | Term::Predecessor(argument) => {
            let (ty, mut constraints) = generate_constraints(context, argument, fresh)?;
            constraints.push((ty, Type::Nat));
            Ok((Type::Nat, constraints))
        }
        Term::IsZero(argument) => {
            let (ty, mut constraints) = generate_constraints(context, argument, fresh)?;
            constraints.push((ty, Type::Nat));
            Ok((Type::Bool, constraints))
        }
        Term::If(guard, then_term, else_term) => {
            let (guard_type, mut constraints) = generate_constraints(context, guard, fresh)?;
            let (then_type, then_constraints) = generate_constraints(context, then_term, fresh)?;
            let (else_type, else_constraints) = generate_constraints(context, else_term, fresh)?;
            constraints.extend(then_constraints);
            constraints.extend(else_constraints);
            constraints.push((guard_type, Type::Bool));
            constraints.push((then_type.clone(), else_type));
            Ok((then_type, constraints))
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
        Term::Reference(argument) => {
            let (ty, constraints) = generate_constraints(context, argument, fresh)?;
            Ok((Type::Reference(Box::new(ty)), constraints))
        }
        Term::Dereference(argument) => {
            let (ty, mut constraints) = generate_constraints(context, argument, fresh)?;
            let result = fresh.fresh();
            constraints.push((ty, Type::Reference(Box::new(result.clone()))));
            Ok((result, constraints))
        }
        Term::Assignment(target, value) => {
            let (target_type, mut constraints) = generate_constraints(context, target, fresh)?;
            let (value_type, value_constraints) = generate_constraints(context, value, fresh)?;
            constraints.extend(value_constraints);
            constraints.push((target_type, Type::Reference(Box::new(value_type))));
            Ok((Type::Unit, constraints))
        }
        Term::Sequence(first, second) => {
            let (first_type, mut constraints) = generate_constraints(context, first, fresh)?;
            let (second_type, second_constraints) = generate_constraints(context, second, fresh)?;
            constraints.extend(second_constraints);
            constraints.push((first_type, Type::Unit));
            Ok((second_type, constraints))
        }
        Term::Let { name, value, body } => {
            let (value_type, mut constraints) = generate_constraints(context, value, fresh)?;
            let mut body_context = context.clone();
            body_context.insert(name.clone(), value_type);
            let (body_type, body_constraints) = generate_constraints(&body_context, body, fresh)?;
            constraints.extend(body_constraints);
            Ok((body_type, constraints))
        }
    }
}
// TAPL-SNIPPET-END: ch22-constraints

// TAPL-SNIPPET-BEGIN: ch22-principal-type
/// Reconstructs the principal type by generating and then unifying constraints.
pub fn principal_type(context: &Context, term: &Term) -> Result<Type, TypeError> {
    let mut fresh = FreshVariables::default();
    for ty in context.values() {
        fresh.reserve_type(ty);
    }
    fresh.reserve_term_annotations(term);
    let (schematic_type, constraints) = generate_constraints(context, term, &mut fresh)?;
    let substitution = unify(constraints)?;
    Ok(apply(&substitution, &schematic_type))
}
// TAPL-SNIPPET-END: ch22-principal-type

// TAPL-SNIPPET-BEGIN: ch22-algorithm-w-support
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeScheme {
    quantified: BTreeSet<String>,
    ty: Type,
}

impl TypeScheme {
    fn monomorphic(ty: Type) -> Self {
        Self {
            quantified: BTreeSet::new(),
            ty,
        }
    }
}

pub type SchemeContext = BTreeMap<String, TypeScheme>;

fn type_variables(ty: &Type) -> BTreeSet<String> {
    let mut variables = BTreeSet::new();
    free_variables(ty, &mut variables);
    variables
}

fn scheme_variables(scheme: &TypeScheme) -> BTreeSet<String> {
    type_variables(&scheme.ty)
        .difference(&scheme.quantified)
        .cloned()
        .collect()
}

fn context_variables(context: &SchemeContext) -> BTreeSet<String> {
    context.values().flat_map(scheme_variables).collect()
}

fn apply_scheme(substitution: &Substitution, scheme: &TypeScheme) -> TypeScheme {
    let filtered = substitution
        .iter()
        .filter(|(name, _)| !scheme.quantified.contains(*name))
        .map(|(name, ty)| (name.clone(), ty.clone()))
        .collect();
    TypeScheme {
        quantified: scheme.quantified.clone(),
        ty: apply(&filtered, &scheme.ty),
    }
}

fn apply_context(substitution: &Substitution, context: &SchemeContext) -> SchemeContext {
    context
        .iter()
        .map(|(name, scheme)| (name.clone(), apply_scheme(substitution, scheme)))
        .collect()
}

fn instantiate(scheme: &TypeScheme, fresh: &mut FreshVariables) -> Type {
    let substitution = scheme
        .quantified
        .iter()
        .map(|name| (name.clone(), fresh.fresh()))
        .collect();
    apply(&substitution, &scheme.ty)
}

fn generalize(context: &SchemeContext, ty: Type) -> TypeScheme {
    let context_variables = context_variables(context);
    let quantified = type_variables(&ty)
        .difference(&context_variables)
        .cloned()
        .collect();
    TypeScheme { quantified, ty }
}
// TAPL-SNIPPET-END: ch22-algorithm-w-support

// TAPL-SNIPPET-BEGIN: ch22-algorithm-w
/// Infers a principal type incrementally, in the style of Algorithm W.
///
/// Each recursive call returns both the type of the subterm and the most
/// general substitution learned while checking it.  Later subterms are
/// checked in a context to which that substitution has already been applied.
#[allow(clippy::too_many_lines)]
pub fn infer_incremental(
    context: &SchemeContext,
    term: &Term,
    fresh: &mut FreshVariables,
) -> Result<(Substitution, Type), TypeError> {
    match term {
        Term::True | Term::False => Ok((Substitution::new(), Type::Bool)),
        Term::Zero => Ok((Substitution::new(), Type::Nat)),
        Term::Unit => Ok((Substitution::new(), Type::Unit)),
        Term::Variable(name) => context
            .get(name)
            .map(|scheme| (Substitution::new(), instantiate(scheme, fresh)))
            .ok_or_else(|| TypeError::UnknownVariable(name.clone())),
        Term::Abstraction {
            parameter,
            annotation,
            body,
        } => {
            let parameter_type = annotation.clone().unwrap_or_else(|| fresh.fresh());
            let mut body_context = context.clone();
            body_context.insert(
                parameter.clone(),
                TypeScheme::monomorphic(parameter_type.clone()),
            );
            let (body_substitution, body_type) = infer_incremental(&body_context, body, fresh)?;
            Ok((
                body_substitution.clone(),
                Type::Arrow(
                    Box::new(apply(&body_substitution, &parameter_type)),
                    Box::new(body_type),
                ),
            ))
        }
        Term::Application(function, argument) => {
            let (function_substitution, function_type) =
                infer_incremental(context, function, fresh)?;
            let argument_context = apply_context(&function_substitution, context);
            let (argument_substitution, argument_type) =
                infer_incremental(&argument_context, argument, fresh)?;
            let result_type = fresh.fresh();
            let application_substitution = unify(vec![(
                apply(&argument_substitution, &function_type),
                Type::Arrow(Box::new(argument_type), Box::new(result_type.clone())),
            )])?;
            let substitution = compose(
                &application_substitution,
                &compose(&argument_substitution, &function_substitution),
            );
            Ok((substitution, apply(&application_substitution, &result_type)))
        }
        Term::Reference(argument) => {
            let (substitution, ty) = infer_incremental(context, argument, fresh)?;
            Ok((substitution, Type::Reference(Box::new(ty))))
        }
        Term::Dereference(argument) => {
            let (substitution, ty) = infer_incremental(context, argument, fresh)?;
            let result = fresh.fresh();
            let dereference = unify(vec![(ty, Type::Reference(Box::new(result.clone())))])?;
            Ok((
                compose(&dereference, &substitution),
                apply(&dereference, &result),
            ))
        }
        Term::Assignment(target, value) => {
            let (target_substitution, target_type) = infer_incremental(context, target, fresh)?;
            let value_context = apply_context(&target_substitution, context);
            let (value_substitution, value_type) = infer_incremental(&value_context, value, fresh)?;
            let assignment = unify(vec![(
                apply(&value_substitution, &target_type),
                Type::Reference(Box::new(value_type)),
            )])?;
            Ok((
                compose(
                    &assignment,
                    &compose(&value_substitution, &target_substitution),
                ),
                Type::Unit,
            ))
        }
        Term::Sequence(first, second) => {
            let (first_substitution, first_type) = infer_incremental(context, first, fresh)?;
            let unit = unify(vec![(first_type, Type::Unit)])?;
            let before_second = compose(&unit, &first_substitution);
            let second_context = apply_context(&before_second, context);
            let (second_substitution, second_type) =
                infer_incremental(&second_context, second, fresh)?;
            Ok((compose(&second_substitution, &before_second), second_type))
        }
        Term::Successor(argument) | Term::Predecessor(argument) => {
            let (substitution, ty) = infer_incremental(context, argument, fresh)?;
            let numeric = unify(vec![(ty, Type::Nat)])?;
            Ok((compose(&numeric, &substitution), Type::Nat))
        }
        Term::IsZero(argument) => {
            let (substitution, ty) = infer_incremental(context, argument, fresh)?;
            let numeric = unify(vec![(ty, Type::Nat)])?;
            Ok((compose(&numeric, &substitution), Type::Bool))
        }
        Term::If(guard, then_term, else_term) => {
            let (guard_substitution, guard_type) = infer_incremental(context, guard, fresh)?;
            let guard_check = unify(vec![(guard_type, Type::Bool)])?;
            let after_guard = compose(&guard_check, &guard_substitution);
            let branch_context = apply_context(&after_guard, context);
            let (then_substitution, then_type) =
                infer_incremental(&branch_context, then_term, fresh)?;
            let else_context = apply_context(&then_substitution, &branch_context);
            let (else_substitution, else_type) =
                infer_incremental(&else_context, else_term, fresh)?;
            let branch_check = unify(vec![(
                apply(&else_substitution, &then_type),
                else_type.clone(),
            )])?;
            let substitution = compose(
                &branch_check,
                &compose(
                    &else_substitution,
                    &compose(&then_substitution, &after_guard),
                ),
            );
            Ok((substitution, apply(&branch_check, &else_type)))
        }
        Term::Let { name, value, body } => {
            let (value_substitution, value_type) = infer_incremental(context, value, fresh)?;
            let value_context = apply_context(&value_substitution, context);
            let value_type = apply(&value_substitution, &value_type);
            let scheme = if is_syntactic_value(value) {
                generalize(&value_context, value_type)
            } else {
                TypeScheme::monomorphic(value_type)
            };
            let mut body_context = value_context;
            body_context.insert(name.clone(), scheme);
            let (body_substitution, body_type) = infer_incremental(&body_context, body, fresh)?;
            Ok((compose(&body_substitution, &value_substitution), body_type))
        }
    }
}
// TAPL-SNIPPET-END: ch22-algorithm-w

// TAPL-SNIPPET-BEGIN: ch22-algorithm-w-value-support
fn is_syntactic_value(term: &Term) -> bool {
    match term {
        Term::True | Term::False | Term::Zero | Term::Unit | Term::Abstraction { .. } => true,
        Term::Successor(argument) => is_syntactic_value(argument),
        Term::Variable(_)
        | Term::Predecessor(_)
        | Term::IsZero(_)
        | Term::If(_, _, _)
        | Term::Application(_, _)
        | Term::Reference(_)
        | Term::Dereference(_)
        | Term::Assignment(_, _)
        | Term::Sequence(_, _)
        | Term::Let { .. } => false,
    }
}
// TAPL-SNIPPET-END: ch22-algorithm-w-value-support

// TAPL-SNIPPET-BEGIN: ch22-let-principal-type
/// Reconstructs a principal type with ML-style let-generalization.
pub fn let_principal_type(context: &SchemeContext, term: &Term) -> Result<Type, TypeError> {
    let mut fresh = FreshVariables::default();
    for scheme in context.values() {
        fresh.reserve_type(&scheme.ty);
    }
    fresh.reserve_term_annotations(term);
    let (substitution, ty) = infer_incremental(context, term, &mut fresh)?;
    Ok(apply(&substitution, &ty))
}
// TAPL-SNIPPET-END: ch22-let-principal-type

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

    #[test]
    fn generated_variables_do_not_collide_with_annotations() {
        let term = Term::Abstraction {
            parameter: "x".into(),
            annotation: Some(Type::Variable("?X0".into())),
            body: Box::new(Term::Application(
                Box::new(variable("x")),
                Box::new(Term::Zero),
            )),
        };
        assert!(principal_type(&Context::new(), &term).is_ok());
    }

    #[test]
    fn reconstructs_arithmetic_and_conditionals() {
        let term = Term::If(
            Box::new(Term::IsZero(Box::new(Term::Predecessor(Box::new(
                Term::Successor(Box::new(Term::Zero)),
            ))))),
            Box::new(Term::True),
            Box::new(Term::False),
        );
        assert_eq!(principal_type(&Context::new(), &term).unwrap(), Type::Bool);
    }

    #[test]
    fn incremental_inference_returns_a_principal_type() {
        let apply_to_zero = Term::Abstraction {
            parameter: "f".into(),
            annotation: None,
            body: Box::new(Term::Application(
                Box::new(variable("f")),
                Box::new(Term::Zero),
            )),
        };
        let ty = let_principal_type(&SchemeContext::new(), &apply_to_zero).unwrap();
        assert!(matches!(
            ty,
            Type::Arrow(domain, result)
                if matches!(*domain, Type::Arrow(ref argument, ref output)
                    if **argument == Type::Nat && **output == *result)
        ));
    }

    #[test]
    fn one_let_bound_identity_has_two_independent_instances() {
        let identity = Term::Abstraction {
            parameter: "x".into(),
            annotation: None,
            body: Box::new(variable("x")),
        };
        let term = Term::Let {
            name: "id".into(),
            value: Box::new(identity),
            body: Box::new(Term::If(
                Box::new(Term::Application(
                    Box::new(variable("id")),
                    Box::new(Term::True),
                )),
                Box::new(Term::IsZero(Box::new(Term::Application(
                    Box::new(variable("id")),
                    Box::new(Term::Zero),
                )))),
                Box::new(Term::False),
            )),
        };
        assert_eq!(
            let_principal_type(&SchemeContext::new(), &term).unwrap(),
            Type::Bool
        );
    }

    #[test]
    fn generalization_does_not_capture_context_variables() {
        let x = Type::Variable("X".into());
        let mut context = SchemeContext::new();
        context.insert(
            "f".into(),
            TypeScheme::monomorphic(Type::Arrow(Box::new(x.clone()), Box::new(x))),
        );
        let term = Term::Let {
            name: "g".into(),
            value: Box::new(variable("f")),
            body: Box::new(Term::Application(
                Box::new(variable("g")),
                Box::new(Term::Zero),
            )),
        };
        let ty = let_principal_type(&context, &term).unwrap();
        assert_eq!(ty, Type::Nat);
    }

    #[test]
    fn value_restriction_rejects_polymorphic_references() {
        let identity = Term::Abstraction {
            parameter: "x".into(),
            annotation: None,
            body: Box::new(variable("x")),
        };
        let successor = Term::Abstraction {
            parameter: "x".into(),
            annotation: Some(Type::Nat),
            body: Box::new(Term::Successor(Box::new(variable("x")))),
        };
        let dangerous = Term::Let {
            name: "r".into(),
            value: Box::new(Term::Reference(Box::new(identity))),
            body: Box::new(Term::Sequence(
                Box::new(Term::Assignment(
                    Box::new(variable("r")),
                    Box::new(successor),
                )),
                Box::new(Term::Application(
                    Box::new(Term::Dereference(Box::new(variable("r")))),
                    Box::new(Term::True),
                )),
            )),
        };
        assert!(matches!(
            let_principal_type(&SchemeContext::new(), &dangerous),
            Err(TypeError::CannotUnify(Type::Nat, Type::Bool)
                | TypeError::CannotUnify(Type::Bool, Type::Nat))
        ));
    }
}
