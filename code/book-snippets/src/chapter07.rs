//! Rust counterparts for the OCaml fragments in Chapter 7.

pub mod basic_term {
    // TAPL-SNIPPET-BEGIN: ch07-term-basic
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Term {
        Var(usize),
        Abs(Box<Term>),
        App(Box<Term>, Box<Term>),
    }
    // TAPL-SNIPPET-END: ch07-term-basic
}

pub mod located_term {
    // TAPL-SNIPPET-BEGIN: ch07-term-info
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Info;

    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Term {
        Var(Info, usize),
        Abs(Info, Box<Term>),
        App(Info, Box<Term>, Box<Term>),
    }
    // TAPL-SNIPPET-END: ch07-term-info
}

pub mod checked_term {
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Info;

    // TAPL-SNIPPET-BEGIN: ch07-term-context-length
    #[derive(Clone, Debug, PartialEq, Eq)]
    pub enum Term {
        Var(Info, usize, usize),
        Abs(Info, Box<Term>),
        App(Info, Box<Term>, Box<Term>),
    }
    // TAPL-SNIPPET-END: ch07-term-context-length
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Info;

// TAPL-SNIPPET-BEGIN: ch07-term-name-hint
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    Var(Info, usize, usize),
    Abs(Info, String, Box<Term>),
    App(Info, Box<Term>, Box<Term>),
}
// TAPL-SNIPPET-END: ch07-term-name-hint

// TAPL-SNIPPET-BEGIN: ch07-context
pub type Context = Vec<(String, Binding)>;
// TAPL-SNIPPET-END: ch07-context

// TAPL-SNIPPET-BEGIN: ch07-binding
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Binding {
    Name,
}
// TAPL-SNIPPET-END: ch07-binding

// TAPL-SNIPPET-BEGIN: ch07-eval-error
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalError {
    // 当前项已经没有可用的求值规则。
    NoRuleApplies,
    // 移位后的索引或上下文长度会变成负数，因而不是合法的 de Bruijn 表示。
    NegativeShift,
    // 项中记录的上下文长度与打印时实际提供的上下文长度不一致。
    BadContextLength { recorded: usize, actual: usize },
    // 变量索引超出了当前上下文的范围。
    UnboundIndex { index: usize, context_len: usize },
}
// TAPL-SNIPPET-END: ch07-eval-error

// TAPL-SNIPPET-BEGIN: ch07-print-helpers
// 优先沿用原项提供的名字提示；若发生冲突，就不断添加撇号。
fn pick_fresh_name(context: &Context, hint: &str) -> (Context, String) {
    let mut name = if hint.is_empty() {
        "x".to_owned()
    } else {
        hint.to_owned()
    };
    while context.iter().any(|(used, _)| used == &name) {
        name.push('\'');
    }
    let mut extended = context.clone();
    extended.insert(0, (name.clone(), Binding::Name));
    (extended, name)
}

// de Bruijn 索引从 0 开始计数，直接对应上下文中的位置。
fn index_to_name(context: &Context, index: usize) -> Result<String, EvalError> {
    context
        .get(index)
        .map(|(name, _)| name.clone())
        .ok_or(EvalError::UnboundIndex {
            index,
            context_len: context.len(),
        })
}
// TAPL-SNIPPET-END: ch07-print-helpers

// TAPL-SNIPPET-BEGIN: ch07-print-term
pub fn print_term(context: &Context, term: &Term) -> Result<String, EvalError> {
    match term {
        Term::Abs(_, hint, body) => {
            let (extended, name) = pick_fresh_name(context, hint);
            Ok(format!("(lambda {name}. {})", print_term(&extended, body)?))
        }
        Term::App(_, function, argument) => Ok(format!(
            "({} {})",
            print_term(context, function)?,
            print_term(context, argument)?
        )),
        Term::Var(_, index, context_len) => {
            if *context_len == context.len() {
                index_to_name(context, *index)
            } else {
                Err(EvalError::BadContextLength {
                    recorded: *context_len,
                    actual: context.len(),
                })
            }
        }
    }
}
// TAPL-SNIPPET-END: ch07-print-term

// TAPL-SNIPPET-BEGIN: ch07-shift
// 索引使用 usize，而位移量可能为负。checked_add_signed 会拒绝小于零或
// 超出 usize 范围的结果；调用处的 `?` 会继续向外传播这个错误。
fn shifted(value: usize, distance: isize) -> Result<usize, EvalError> {
    value
        .checked_add_signed(distance)
        .ok_or(EvalError::NegativeShift)
}

pub fn term_shift(distance: isize, term: &Term) -> Result<Term, EvalError> {
    fn walk(distance: isize, cutoff: usize, term: &Term) -> Result<Term, EvalError> {
        match term {
            Term::Var(info, index, context_len) => Ok(Term::Var(
                *info,
                if *index >= cutoff {
                    shifted(*index, distance)?
                } else {
                    *index
                },
                shifted(*context_len, distance)?,
            )),
            Term::Abs(info, hint, body) => Ok(Term::Abs(
                *info,
                hint.clone(),
                Box::new(walk(distance, cutoff + 1, body)?),
            )),
            Term::App(info, function, argument) => Ok(Term::App(
                *info,
                Box::new(walk(distance, cutoff, function)?),
                Box::new(walk(distance, cutoff, argument)?),
            )),
        }
    }

    walk(distance, 0, term)
}
// TAPL-SNIPPET-END: ch07-shift

// TAPL-SNIPPET-BEGIN: ch07-substitute
pub fn term_substitute(
    variable: usize,
    replacement: &Term,
    term: &Term,
) -> Result<Term, EvalError> {
    fn walk(
        variable: usize,
        replacement: &Term,
        cutoff: usize,
        term: &Term,
    ) -> Result<Term, EvalError> {
        match term {
            Term::Var(_, index, _) if *index == variable + cutoff => {
                // 替换项将被放到已经进入的 `cutoff` 层抽象之下；先上移其中的
                // 自由变量，避免它们被这几层抽象意外捕获。
                term_shift(isize::try_from(cutoff).expect("cutoff fits"), replacement)
            }
            Term::Var(info, index, context_len) => Ok(Term::Var(*info, *index, *context_len)),
            Term::Abs(info, hint, body) => Ok(Term::Abs(
                *info,
                hint.clone(),
                Box::new(walk(variable, replacement, cutoff + 1, body)?),
            )),
            Term::App(info, function, argument) => Ok(Term::App(
                *info,
                Box::new(walk(variable, replacement, cutoff, function)?),
                Box::new(walk(variable, replacement, cutoff, argument)?),
            )),
        }
    }

    walk(variable, replacement, 0, term)
}
// TAPL-SNIPPET-END: ch07-substitute

// TAPL-SNIPPET-BEGIN: ch07-substitute-top
pub fn term_substitute_top(replacement: &Term, body: &Term) -> Result<Term, EvalError> {
    // 为即将消去的参数绑定腾出一个索引位置。
    let lifted = term_shift(1, replacement)?;
    let substituted = term_substitute(0, &lifted, body)?;
    // 替换完成后，移除这个参数绑定占用的索引位置。
    term_shift(-1, &substituted)
}
// TAPL-SNIPPET-END: ch07-substitute-top

// TAPL-SNIPPET-BEGIN: ch07-is-value
pub const fn is_value(_context: &Context, term: &Term) -> bool {
    matches!(term, Term::Abs(_, _, _))
}
// TAPL-SNIPPET-END: ch07-is-value

// TAPL-SNIPPET-BEGIN: ch07-eval1
pub fn eval1(context: &Context, term: &Term) -> Result<Term, EvalError> {
    match term {
        Term::App(_, function, argument)
            if matches!(function.as_ref(), Term::Abs(_, _, _)) && is_value(context, argument) =>
        {
            let Term::Abs(_, _, body) = function.as_ref() else {
                unreachable!();
            };
            term_substitute_top(argument, body)
        }
        Term::App(info, function, argument) if is_value(context, function) => Ok(Term::App(
            *info,
            function.clone(),
            Box::new(eval1(context, argument)?),
        )),
        Term::App(info, function, argument) => Ok(Term::App(
            *info,
            Box::new(eval1(context, function)?),
            argument.clone(),
        )),
        Term::Var(_, _, _) | Term::Abs(_, _, _) => Err(EvalError::NoRuleApplies),
    }
}
// TAPL-SNIPPET-END: ch07-eval1

// TAPL-SNIPPET-BEGIN: ch07-eval
pub fn eval(context: &Context, term: Term) -> Result<Term, EvalError> {
    match eval1(context, &term) {
        Ok(next) => eval(context, next),
        Err(EvalError::NoRuleApplies) => Ok(term),
        Err(error) => Err(error),
    }
}
// TAPL-SNIPPET-END: ch07-eval

// TAPL-SNIPPET-BEGIN: sol-translator-07-eval-big
pub fn eval_big(context: &Context, term: &Term) -> Result<Term, EvalError> {
    match term {
        Term::Abs(_, _, _) => Ok(term.clone()),
        Term::App(_, function, argument) => {
            let function_value = eval_big(context, function)?;
            let argument_value = eval_big(context, argument)?;
            let Term::Abs(_, _, body) = function_value else {
                return Err(EvalError::NoRuleApplies);
            };
            let reduced = term_substitute_top(&argument_value, &body)?;
            eval_big(context, &reduced)
        }
        Term::Var(_, _, _) => Err(EvalError::NoRuleApplies),
    }
}
// TAPL-SNIPPET-END: sol-translator-07-eval-big

#[cfg(test)]
mod tests {
    use super::*;

    fn var(index: usize, context_len: usize) -> Term {
        Term::Var(Info, index, context_len)
    }

    fn abs(hint: &str, body: Term) -> Term {
        Term::Abs(Info, hint.into(), Box::new(body))
    }

    fn app(function: Term, argument: Term) -> Term {
        Term::App(Info, Box::new(function), Box::new(argument))
    }

    #[test]
    fn small_and_big_step_agree() {
        let identity = abs("x", var(0, 1));
        let argument = abs("z", var(0, 1));
        let term = app(identity, argument.clone());
        assert_eq!(eval(&Vec::new(), term.clone()), Ok(argument.clone()));
        assert_eq!(eval_big(&Vec::new(), &term), Ok(argument));
    }

    #[test]
    fn printing_uses_fresh_hints() {
        let term = abs("x", abs("x", var(0, 2)));
        assert_eq!(
            print_term(&Vec::new(), &term),
            Ok("(lambda x. (lambda x'. x'))".into())
        );
    }

    #[test]
    fn signed_shifts_are_checked() {
        assert_eq!(shifted(1, -1), Ok(0));
        assert_eq!(shifted(0, -1), Err(EvalError::NegativeShift));

        let free_variable = var(1, 2);
        assert_eq!(term_shift(-1, &free_variable), Ok(var(0, 1)));
        assert_eq!(
            term_shift(-2, &free_variable),
            Err(EvalError::NegativeShift)
        );
    }
}
