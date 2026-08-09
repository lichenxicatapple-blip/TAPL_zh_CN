//! Rust counterparts for the OCaml datatype fragments in Chapter 11.

pub mod schematic_variant {
    // TAPL-SNIPPET-BEGIN: ch11-variant-schematic
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Variant<T1, T2> {
        L1(T1),
        L2(T2),
    }
    // TAPL-SNIPPET-END: ch11-variant-schematic
}

// TAPL-SNIPPET-BEGIN: ch11-weekday
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
}
// TAPL-SNIPPET-END: ch11-weekday

pub type Nat = u64;

// TAPL-SNIPPET-BEGIN: ch11-nat-list
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NatList {
    Nil,
    Cons(Nat, Box<NatList>),
}
// TAPL-SNIPPET-END: ch11-nat-list

// TAPL-SNIPPET-BEGIN: ch11-generic-list
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum List<T> {
    Nil,
    Cons(T, Box<List<T>>),
}
// TAPL-SNIPPET-END: ch11-generic-list

pub mod let_expression {
    // TAPL-SNIPPET-BEGIN: sol-author-11-let-types
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Type {
        Unit,
        Arrow(Box<Type>, Box<Type>),
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Term {
        Unit,
        Var(usize),
        Abs(Type, Box<Term>),
        App(Box<Term>, Box<Term>),
        Let(Box<Term>, Box<Term>),
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Error {
        NoRuleApplies,
        UnboundVariable(usize),
        ExpectedFunction(Type),
        ParameterMismatch { expected: Type, actual: Type },
    }

    fn is_value(term: &Term) -> bool {
        matches!(term, Term::Unit | Term::Abs(_, _))
    }
    // TAPL-SNIPPET-END: sol-author-11-let-types

    // TAPL-SNIPPET-BEGIN: sol-author-11-let-eval1
    pub fn eval1(term: &Term) -> Result<Term, Error> {
        match term {
            // E-AppAbs：函数与参数都已成为值，执行一次顶层替换。
            Term::App(function, argument)
                if matches!(function.as_ref(), Term::Abs(_, _)) && is_value(argument) =>
            {
                let Term::Abs(_, body) = function.as_ref() else {
                    unreachable!("the guard above requires an abstraction")
                };
                Ok(substitute_top(argument, body))
            }
            // E-App1：函数位置尚未成为值，先让函数走一步。
            Term::App(function, argument) if !is_value(function) => Ok(Term::App(
                Box::new(eval1(function)?),
                Box::new((**argument).clone()),
            )),
            // E-App2：函数已经是值，接着求值参数。
            Term::App(function, argument) => Ok(Term::App(
                Box::new((**function).clone()),
                Box::new(eval1(argument)?),
            )),
            // E-LetV：绑定项已经是值，代入 let 主体。
            Term::Let(bound, body) if is_value(bound) => Ok(substitute_top(bound, body)),
            // E-Let：绑定项尚未成为值，只让它先走一步。
            Term::Let(bound, body) => Ok(Term::Let(
                Box::new(eval1(bound)?),
                Box::new((**body).clone()),
            )),
            _ => Err(Error::NoRuleApplies),
        }
    }
    // TAPL-SNIPPET-END: sol-author-11-let-eval1

    // TAPL-SNIPPET-BEGIN: sol-author-11-let-shift
    // 把代入项放进函数体或 let 主体时，需要按当前绑定深度移动其中的自由变量。
    fn shift(term: &Term, distance: isize, cutoff: usize) -> Term {
        match term {
            Term::Unit => Term::Unit,
            Term::Var(index) if *index >= cutoff => {
                Term::Var(index.checked_add_signed(distance).expect("valid shift"))
            }
            Term::Var(index) => Term::Var(*index),
            Term::Abs(parameter, body) => Term::Abs(
                parameter.clone(),
                Box::new(shift(body, distance, cutoff + 1)),
            ),
            Term::App(function, argument) => Term::App(
                Box::new(shift(function, distance, cutoff)),
                Box::new(shift(argument, distance, cutoff)),
            ),
            Term::Let(bound, body) => Term::Let(
                Box::new(shift(bound, distance, cutoff)),
                Box::new(shift(body, distance, cutoff + 1)),
            ),
        }
    }
    // TAPL-SNIPPET-END: sol-author-11-let-shift

    // TAPL-SNIPPET-BEGIN: sol-author-11-let-substitution
    fn substitute(term: &Term, replacement: &Term, variable: usize, cutoff: usize) -> Term {
        match term {
            Term::Unit => Term::Unit,
            Term::Var(index) if *index == variable + cutoff => {
                let distance = isize::try_from(cutoff).expect("cutoff fits in isize");
                shift(replacement, distance, 0)
            }
            Term::Var(index) => Term::Var(*index),
            Term::Abs(parameter, body) => Term::Abs(
                parameter.clone(),
                Box::new(substitute(body, replacement, variable, cutoff + 1)),
            ),
            Term::App(function, argument) => Term::App(
                Box::new(substitute(function, replacement, variable, cutoff)),
                Box::new(substitute(argument, replacement, variable, cutoff)),
            ),
            Term::Let(bound, body) => Term::Let(
                Box::new(substitute(bound, replacement, variable, cutoff)),
                Box::new(substitute(body, replacement, variable, cutoff + 1)),
            ),
        }
    }

    fn substitute_top(replacement: &Term, body: &Term) -> Term {
        let lifted = shift(replacement, 1, 0);
        let substituted = substitute(body, &lifted, 0, 0);
        shift(&substituted, -1, 0)
    }
    // TAPL-SNIPPET-END: sol-author-11-let-substitution

    // TAPL-SNIPPET-BEGIN: sol-author-11-let-typeof
    pub fn type_of(context: &[Type], term: &Term) -> Result<Type, Error> {
        match term {
            Term::Unit => Ok(Type::Unit),
            Term::Var(index) => context
                .get(*index)
                .cloned()
                .ok_or(Error::UnboundVariable(*index)),
            Term::Abs(parameter, body) => {
                let mut body_context = context.to_vec();
                body_context.insert(0, parameter.clone());
                let result = type_of(&body_context, body)?;
                Ok(Type::Arrow(Box::new(parameter.clone()), Box::new(result)))
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
                    other @ Type::Unit => Err(Error::ExpectedFunction(other)),
                }
            }
            // T-Let：先取得绑定项的类型，再用它扩展主体的上下文。
            Term::Let(bound, body) => {
                let bound_type = type_of(context, bound)?;
                let mut body_context = context.to_vec();
                body_context.insert(0, bound_type);
                type_of(&body_context, body)
            }
        }
    }
    // TAPL-SNIPPET-END: sol-author-11-let-typeof
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recursive_lists_construct_values() {
        let naturals = NatList::Cons(1, Box::new(NatList::Nil));
        assert_eq!(naturals, NatList::Cons(1, Box::new(NatList::Nil)));

        let words = List::Cons("tapl", Box::new(List::Nil));
        assert_eq!(words, List::Cons("tapl", Box::new(List::Nil)));
    }

    #[test]
    fn let_evaluation_and_typing_follow_the_book_rules() {
        use let_expression::{Term, Type, eval1, type_of};

        let identity = Term::Abs(Type::Unit, Box::new(Term::Var(0)));
        let term = Term::Let(Box::new(identity.clone()), Box::new(Term::Var(0)));

        assert_eq!(eval1(&term), Ok(identity.clone()));
        assert_eq!(
            type_of(&[], &term),
            Ok(Type::Arrow(Box::new(Type::Unit), Box::new(Type::Unit)))
        );

        let delayed = Term::Let(
            Box::new(Term::Let(Box::new(Term::Unit), Box::new(Term::Var(0)))),
            Box::new(Term::Var(0)),
        );
        assert_eq!(
            eval1(&delayed),
            Ok(Term::Let(Box::new(Term::Unit), Box::new(Term::Var(0))))
        );

        let applied_identity = Term::App(Box::new(identity), Box::new(Term::Unit));
        let let_after_one_step = Term::Let(
            Box::new(applied_identity),
            Box::new(Term::Abs(Type::Unit, Box::new(Term::Var(1)))),
        );
        let reduced_bound = eval1(&let_after_one_step).expect("the bound application reduces");
        assert_eq!(
            reduced_bound,
            Term::Let(
                Box::new(Term::Unit),
                Box::new(Term::Abs(Type::Unit, Box::new(Term::Var(1))))
            )
        );
        assert_eq!(
            eval1(&reduced_bound),
            Ok(Term::Abs(Type::Unit, Box::new(Term::Unit)))
        );
    }
}
