//! Rust counterparts for the OCaml fragments in Chapter 4.

// TAPL-SNIPPET-BEGIN: ch04-term
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Info {
    pub line: usize,
    pub column: usize,
}

const DUMMY_INFO: Info = Info { line: 0, column: 0 };

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    True(Info),
    False(Info),
    If(Info, Box<Term>, Box<Term>, Box<Term>),
    Zero(Info),
    Succ(Info, Box<Term>),
    Pred(Info, Box<Term>),
    IsZero(Info, Box<Term>),
}
// TAPL-SNIPPET-END: ch04-term

// TAPL-SNIPPET-BEGIN: ch04-is-numeric-value
pub fn is_numeric_value(term: &Term) -> bool {
    match term {
        Term::Zero(_) => true,
        Term::Succ(_, inner) => is_numeric_value(inner),
        _ => false,
    }
}
// TAPL-SNIPPET-END: ch04-is-numeric-value

// TAPL-SNIPPET-BEGIN: ch04-is-value
pub fn is_value(term: &Term) -> bool {
    matches!(term, Term::True(_) | Term::False(_)) || is_numeric_value(term)
}
// TAPL-SNIPPET-END: ch04-is-value

// TAPL-SNIPPET-BEGIN: ch04-no-rule-applies
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NoRuleApplies;

pub type StepResult = Result<Term, NoRuleApplies>;
// TAPL-SNIPPET-END: ch04-no-rule-applies

// TAPL-SNIPPET-BEGIN: ch04-eval1
pub fn eval1(term: &Term) -> StepResult {
    match term {
        Term::If(_, guard, then_term, _) if matches!(guard.as_ref(), Term::True(_)) => {
            Ok((**then_term).clone())
        }
        Term::If(_, guard, _, else_term) if matches!(guard.as_ref(), Term::False(_)) => {
            Ok((**else_term).clone())
        }
        Term::If(info, guard, then_term, else_term) => Ok(Term::If(
            *info,
            Box::new(eval1(guard)?),
            then_term.clone(),
            else_term.clone(),
        )),
        Term::Succ(info, inner) => Ok(Term::Succ(*info, Box::new(eval1(inner)?))),
        Term::Pred(_, inner) if matches!(inner.as_ref(), Term::Zero(_)) => {
            Ok(Term::Zero(DUMMY_INFO))
        }
        Term::Pred(_, inner) => {
            if let Term::Succ(_, numeric) = inner.as_ref()
                && is_numeric_value(numeric)
            {
                return Ok((**numeric).clone());
            }
            Ok(Term::Pred(DUMMY_INFO, Box::new(eval1(inner)?)))
        }
        Term::IsZero(_, inner) if matches!(inner.as_ref(), Term::Zero(_)) => {
            Ok(Term::True(DUMMY_INFO))
        }
        Term::IsZero(_, inner) => {
            if let Term::Succ(_, numeric) = inner.as_ref()
                && is_numeric_value(numeric)
            {
                return Ok(Term::False(DUMMY_INFO));
            }
            Ok(Term::IsZero(DUMMY_INFO, Box::new(eval1(inner)?)))
        }
        Term::True(_) | Term::False(_) | Term::Zero(_) => Err(NoRuleApplies),
    }
}
// TAPL-SNIPPET-END: ch04-eval1

// TAPL-SNIPPET-BEGIN: ch04-eval
pub fn eval(term: Term) -> Term {
    match eval1(&term) {
        Ok(next) => eval(next),
        Err(NoRuleApplies) => term,
    }
}
// TAPL-SNIPPET-END: ch04-eval

// TAPL-SNIPPET-BEGIN: sol-author-04-eval
pub fn eval_without_exception_handler(mut term: Term) -> Term {
    loop {
        match eval1(&term) {
            Ok(next) => term = next,
            Err(NoRuleApplies) => return term,
        }
    }
}
// TAPL-SNIPPET-END: sol-author-04-eval

// TAPL-SNIPPET-BEGIN: sol-translator-04-eval-big
pub fn eval_big(term: &Term) -> StepResult {
    match term {
        Term::True(_) | Term::False(_) | Term::Zero(_) => Ok(term.clone()),
        Term::If(_, guard, then_term, else_term) => match eval_big(guard)? {
            Term::True(_) => eval_big(then_term),
            Term::False(_) => eval_big(else_term),
            _ => Err(NoRuleApplies),
        },
        Term::Succ(info, inner) => {
            let value = eval_big(inner)?;
            is_numeric_value(&value)
                .then(|| Term::Succ(*info, Box::new(value)))
                .ok_or(NoRuleApplies)
        }
        Term::Pred(_, inner) => match eval_big(inner)? {
            Term::Zero(_) => Ok(Term::Zero(DUMMY_INFO)),
            Term::Succ(_, numeric) if is_numeric_value(&numeric) => Ok(*numeric),
            _ => Err(NoRuleApplies),
        },
        Term::IsZero(_, inner) => match eval_big(inner)? {
            Term::Zero(_) => Ok(Term::True(DUMMY_INFO)),
            Term::Succ(_, numeric) if is_numeric_value(&numeric) => Ok(Term::False(DUMMY_INFO)),
            _ => Err(NoRuleApplies),
        },
    }
}
// TAPL-SNIPPET-END: sol-translator-04-eval-big

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(term: Term) -> Box<Term> {
        Box::new(term)
    }

    #[test]
    fn small_and_big_step_agree() {
        let term = Term::Pred(
            DUMMY_INFO,
            boxed(Term::Succ(
                DUMMY_INFO,
                boxed(Term::Pred(DUMMY_INFO, boxed(Term::Zero(DUMMY_INFO)))),
            )),
        );
        let small = eval(term.clone());
        assert_eq!(small, Term::Zero(DUMMY_INFO));
        assert_eq!(eval_big(&term), Ok(small.clone()));
        assert_eq!(eval_without_exception_handler(term), small);
    }

    #[test]
    fn stuck_terms_report_no_rule() {
        let stuck = Term::Pred(DUMMY_INFO, boxed(Term::True(DUMMY_INFO)));
        assert_eq!(eval_big(&stuck), Err(NoRuleApplies));
        assert_eq!(eval(stuck.clone()), stuck);
    }
}
