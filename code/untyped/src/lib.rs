//! Untyped lambda-calculus evaluator for TAPL Chapters 5 through 7.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    Var { index: usize, context_len: usize },
    Abs { hint: String, body: Box<Term> },
    App(Box<Term>, Box<Term>),
}

impl Term {
    #[must_use]
    pub const fn is_value(&self) -> bool {
        matches!(self, Self::Abs { .. })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EvalError {
    NegativeIndex { value: usize, shift: isize },
    BadContextLength { recorded: usize, actual: usize },
    UnboundIndex { index: usize, context_len: usize },
}

impl fmt::Display for EvalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeIndex { value, shift } => {
                write!(
                    formatter,
                    "shifting {value} by {shift} would make it negative"
                )
            }
            Self::BadContextLength { recorded, actual } => write!(
                formatter,
                "term records context length {recorded}, but the actual length is {actual}"
            ),
            Self::UnboundIndex { index, context_len } => write!(
                formatter,
                "de Bruijn index {index} is outside context of length {context_len}"
            ),
        }
    }
}

impl std::error::Error for EvalError {}

fn shifted(value: usize, distance: isize) -> Result<usize, EvalError> {
    value
        .checked_add_signed(distance)
        .ok_or(EvalError::NegativeIndex {
            value,
            shift: distance,
        })
}

/// Shift the free variables in `term` by `distance` above `cutoff`.
///
/// # Errors
///
/// Returns [`EvalError::NegativeIndex`] if a negative shift would produce a
/// negative index or context length.
pub fn shift_above(distance: isize, cutoff: usize, term: &Term) -> Result<Term, EvalError> {
    fn walk(distance: isize, cutoff: usize, term: &Term) -> Result<Term, EvalError> {
        match term {
            Term::Var { index, context_len } => Ok(Term::Var {
                index: if *index >= cutoff {
                    shifted(*index, distance)?
                } else {
                    *index
                },
                context_len: shifted(*context_len, distance)?,
            }),
            Term::Abs { hint, body } => Ok(Term::Abs {
                hint: hint.clone(),
                body: Box::new(walk(distance, cutoff + 1, body)?),
            }),
            Term::App(function, argument) => Ok(Term::App(
                Box::new(walk(distance, cutoff, function)?),
                Box::new(walk(distance, cutoff, argument)?),
            )),
        }
    }
    walk(distance, cutoff, term)
}

/// Shift every free variable in `term` by `distance`.
///
/// # Errors
///
/// Returns [`EvalError::NegativeIndex`] if a negative shift would produce a
/// negative index or context length.
pub fn shift(distance: isize, term: &Term) -> Result<Term, EvalError> {
    shift_above(distance, 0, term)
}

/// Substitute `replacement` for the free variable numbered `variable`.
///
/// # Errors
///
/// Returns [`EvalError::NegativeIndex`] if an internal shift would produce a
/// negative index or context length.
pub fn substitute(variable: usize, replacement: &Term, term: &Term) -> Result<Term, EvalError> {
    fn walk(
        variable: usize,
        replacement: &Term,
        cutoff: usize,
        term: &Term,
    ) -> Result<Term, EvalError> {
        match term {
            Term::Var { index, .. } if *index == variable + cutoff => shift(
                isize::try_from(cutoff).expect("cutoff fits in isize"),
                replacement,
            ),
            Term::Var { index, context_len } => Ok(Term::Var {
                index: *index,
                context_len: *context_len,
            }),
            Term::Abs { hint, body } => Ok(Term::Abs {
                hint: hint.clone(),
                body: Box::new(walk(variable, replacement, cutoff + 1, body)?),
            }),
            Term::App(function, argument) => Ok(Term::App(
                Box::new(walk(variable, replacement, cutoff, function)?),
                Box::new(walk(variable, replacement, cutoff, argument)?),
            )),
        }
    }
    walk(variable, replacement, 0, term)
}

/// Perform the shift-substitute-shift sequence used by beta reduction.
///
/// # Errors
///
/// Returns [`EvalError::NegativeIndex`] if the supplied terms violate the
/// scoping invariant needed for the final negative shift.
pub fn substitute_top(replacement: &Term, body: &Term) -> Result<Term, EvalError> {
    let lifted = shift(1, replacement)?;
    let substituted = substitute(0, &lifted, body)?;
    shift(-1, &substituted)
}

/// Apply one call-by-value evaluation rule.
///
/// # Errors
///
/// Returns [`EvalError`] when shifting or substitution detects a broken
/// de Bruijn scoping invariant.
pub fn step(term: &Term) -> Result<Option<Term>, EvalError> {
    match term {
        Term::App(function, argument)
            if matches!(function.as_ref(), Term::Abs { .. }) && argument.is_value() =>
        {
            let Term::Abs { body, .. } = function.as_ref() else {
                unreachable!("guard establishes that the function is an abstraction");
            };
            Ok(Some(substitute_top(argument, body)?))
        }
        Term::App(function, argument) if function.is_value() => {
            Ok(step(argument)?.map(|next| Term::App(function.clone(), Box::new(next))))
        }
        Term::App(function, argument) => {
            Ok(step(function)?.map(|next| Term::App(Box::new(next), argument.clone())))
        }
        Term::Var { .. } | Term::Abs { .. } => Ok(None),
    }
}

/// Repeatedly apply [`step`] until no evaluation rule is applicable.
///
/// # Errors
///
/// Returns [`EvalError`] when an intermediate term violates a de Bruijn
/// scoping invariant.
pub fn eval(mut term: Term) -> Result<Term, EvalError> {
    while let Some(next) = step(&term)? {
        term = next;
    }
    Ok(term)
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Token {
    Lambda,
    Identifier(String),
    Dot,
    Slash,
    LeftParen,
    RightParen,
    Semicolon,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub offset: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "at byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for ParseError {}

fn lex(source: &str) -> Result<Vec<(Token, usize)>, ParseError> {
    let bytes = source.as_bytes();
    let mut tokens = Vec::new();
    let mut offset = 0;

    while offset < bytes.len() {
        if bytes[offset].is_ascii_whitespace() {
            offset += 1;
            continue;
        }
        if source[offset..].starts_with("/*") {
            let Some(end) = source[offset + 2..].find("*/") else {
                return Err(ParseError {
                    offset,
                    message: "unterminated block comment".into(),
                });
            };
            offset += end + 4;
            continue;
        }

        let token_offset = offset;
        let token = match bytes[offset] {
            b'(' => {
                offset += 1;
                Token::LeftParen
            }
            b')' => {
                offset += 1;
                Token::RightParen
            }
            b'.' => {
                offset += 1;
                Token::Dot
            }
            b'/' => {
                offset += 1;
                Token::Slash
            }
            b';' => {
                offset += 1;
                Token::Semicolon
            }
            byte if byte.is_ascii_alphabetic() || byte == b'_' => {
                offset += 1;
                while offset < bytes.len()
                    && (bytes[offset].is_ascii_alphanumeric()
                        || matches!(bytes[offset], b'_' | b'\''))
                {
                    offset += 1;
                }
                match &source[token_offset..offset] {
                    "lambda" => Token::Lambda,
                    name => Token::Identifier(name.into()),
                }
            }
            byte => {
                return Err(ParseError {
                    offset,
                    message: format!("unexpected character `{}`", char::from(byte)),
                });
            }
        };
        tokens.push((token, token_offset));
    }
    Ok(tokens)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Statement {
    Bind(String),
    Eval(Term),
}

struct Parser {
    tokens: Vec<(Token, usize)>,
    cursor: usize,
    source_len: usize,
    context: Vec<String>,
}

impl Parser {
    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.cursor).map(|(token, _)| token)
    }

    fn peek(&self, distance: usize) -> Option<&Token> {
        self.tokens
            .get(self.cursor + distance)
            .map(|(token, _)| token)
    }

    fn offset(&self) -> usize {
        self.tokens
            .get(self.cursor)
            .map_or(self.source_len, |(_, offset)| *offset)
    }

    fn consume(&mut self, expected: &Token) -> Result<(), ParseError> {
        if self.current() == Some(expected) {
            self.cursor += 1;
            Ok(())
        } else {
            Err(ParseError {
                offset: self.offset(),
                message: format!("expected {expected:?}, found {:?}", self.current()),
            })
        }
    }

    fn identifier(&mut self) -> Result<String, ParseError> {
        let Some(Token::Identifier(name)) = self.current() else {
            return Err(ParseError {
                offset: self.offset(),
                message: format!("expected an identifier, found {:?}", self.current()),
            });
        };
        let name = name.clone();
        self.cursor += 1;
        Ok(name)
    }

    fn term(&mut self) -> Result<Term, ParseError> {
        if self.current() == Some(&Token::Lambda) {
            self.cursor += 1;
            let hint = self.identifier()?;
            self.consume(&Token::Dot)?;
            self.context.insert(0, hint.clone());
            let body_result = self.term();
            self.context.remove(0);
            return Ok(Term::Abs {
                hint,
                body: Box::new(body_result?),
            });
        }
        self.application()
    }

    fn atom_starts(&self) -> bool {
        matches!(
            self.current(),
            Some(Token::Identifier(_) | Token::LeftParen)
        )
    }

    fn application(&mut self) -> Result<Term, ParseError> {
        let mut term = self.atom()?;
        while self.atom_starts() {
            term = Term::App(Box::new(term), Box::new(self.atom()?));
        }
        Ok(term)
    }

    fn atom(&mut self) -> Result<Term, ParseError> {
        match self.current() {
            Some(Token::Identifier(_)) => {
                let offset = self.offset();
                let name = self.identifier()?;
                let Some(index) = self.context.iter().position(|entry| entry == &name) else {
                    return Err(ParseError {
                        offset,
                        message: format!("unbound variable `{name}`"),
                    });
                };
                Ok(Term::Var {
                    index,
                    context_len: self.context.len(),
                })
            }
            Some(Token::LeftParen) => {
                self.cursor += 1;
                let term = self.term()?;
                self.consume(&Token::RightParen)?;
                Ok(term)
            }
            token => Err(ParseError {
                offset: self.offset(),
                message: format!("expected a variable or parenthesized term, found {token:?}"),
            }),
        }
    }
}

/// Parse declarations and semicolon-terminated lambda terms.
///
/// # Errors
///
/// Returns a [`ParseError`] for malformed syntax, duplicate declarations, or
/// variables that are not bound by an abstraction or top-level declaration.
pub fn parse_program(source: &str) -> Result<Vec<Statement>, ParseError> {
    let mut parser = Parser {
        tokens: lex(source)?,
        cursor: 0,
        source_len: source.len(),
        context: Vec::new(),
    };
    let mut statements = Vec::new();

    while parser.current().is_some() {
        if matches!(parser.current(), Some(Token::Identifier(_)))
            && parser.peek(1) == Some(&Token::Slash)
        {
            let name = parser.identifier()?;
            parser.consume(&Token::Slash)?;
            parser.consume(&Token::Semicolon)?;
            if parser.context.iter().any(|entry| entry == &name) {
                return Err(ParseError {
                    offset: parser.offset(),
                    message: format!("duplicate top-level binding `{name}`"),
                });
            }
            parser.context.insert(0, name.clone());
            statements.push(Statement::Bind(name));
        } else {
            let term = parser.term()?;
            parser.consume(&Token::Semicolon)?;
            statements.push(Statement::Eval(term));
        }
    }
    Ok(statements)
}

fn fresh_name(context: &[String], hint: &str) -> String {
    let mut candidate = if hint.is_empty() {
        "x".to_owned()
    } else {
        hint.to_owned()
    };
    while context.iter().any(|name| name == &candidate) {
        candidate.push('\'');
    }
    candidate
}

/// Render a nameless term using names from `global_context` and abstraction
/// hints.
///
/// # Errors
///
/// Returns [`EvalError::BadContextLength`] or [`EvalError::UnboundIndex`] when
/// the redundant context information on a variable exposes a broken invariant.
pub fn print_term(term: &Term, global_context: &[String]) -> Result<String, EvalError> {
    fn write_term(term: &Term, context: &mut Vec<String>) -> Result<String, EvalError> {
        match term {
            Term::Var { index, context_len } => {
                if *context_len != context.len() {
                    return Err(EvalError::BadContextLength {
                        recorded: *context_len,
                        actual: context.len(),
                    });
                }
                context.get(*index).cloned().ok_or(EvalError::UnboundIndex {
                    index: *index,
                    context_len: context.len(),
                })
            }
            Term::Abs { hint, body } => {
                let name = fresh_name(context, hint);
                context.insert(0, name.clone());
                let body = write_term(body, context)?;
                context.remove(0);
                Ok(format!("(lambda {name}. {body})"))
            }
            Term::App(function, argument) => Ok(format!(
                "({} {})",
                write_term(function, context)?,
                write_term(argument, context)?
            )),
        }
    }

    let mut context = global_context.to_vec();
    write_term(term, &mut context)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn var(index: usize, context_len: usize) -> Term {
        Term::Var { index, context_len }
    }

    fn abs(hint: &str, body: Term) -> Term {
        Term::Abs {
            hint: hint.into(),
            body: Box::new(body),
        }
    }

    fn app(function: Term, argument: Term) -> Term {
        Term::App(Box::new(function), Box::new(argument))
    }

    #[test]
    fn official_examples_parse_evaluate_and_print() {
        let source = include_str!("../../../source/official-code/extracted/untyped/test.f");
        let statements = parse_program(source).expect("official test file should parse");
        assert_eq!(statements.len(), 4);

        let mut globals = Vec::new();
        let mut rendered = Vec::new();
        for statement in statements {
            match statement {
                Statement::Bind(name) => globals.insert(0, name),
                Statement::Eval(term) => {
                    let value = eval(term).expect("evaluation should preserve invariants");
                    rendered.push(
                        print_term(&value, &globals).expect("printing should validate contexts"),
                    );
                }
            }
        }
        assert_eq!(
            rendered,
            vec!["x", "(lambda x'. x')", "(lambda x'. (x' x'))",]
        );
    }

    #[test]
    fn shifting_respects_the_cutoff_and_context_lengths() {
        let term = abs("x", app(var(0, 2), var(1, 2)));
        assert_eq!(
            shift(1, &term).expect("positive shifts are defined"),
            abs("x", app(var(0, 3), var(2, 3)))
        );
    }

    #[test]
    fn substitution_avoids_capture() {
        let body = abs("y", app(var(1, 2), var(0, 2)));
        let replacement = abs("z", var(0, 1));
        let result = substitute_top(&replacement, &body).expect("well-scoped substitution");
        assert_eq!(
            print_term(&result, &[]).expect("result should be well scoped"),
            "(lambda y. ((lambda z. z) y))"
        );
    }

    #[test]
    fn call_by_value_reduces_the_argument_before_beta_reduction() {
        let identity = abs("x", var(0, 1));
        let argument = app(identity.clone(), abs("z", var(0, 1)));
        let term = app(identity, argument);
        let value = eval(term).expect("well-scoped evaluation");
        assert_eq!(
            print_term(&value, &[]).expect("value should print"),
            "(lambda z. z)"
        );
    }

    #[test]
    fn invalid_negative_shift_and_bad_context_are_reported() {
        assert!(matches!(
            shift(-1, &var(0, 0)),
            Err(EvalError::NegativeIndex { .. })
        ));
        assert!(matches!(
            print_term(&var(0, 2), &["x".into()]),
            Err(EvalError::BadContextLength { .. })
        ));
    }

    #[test]
    fn parser_rejects_unbound_variables() {
        let error = parse_program("lambda x. y;").expect_err("y has no binding");
        assert!(error.message.contains("unbound variable `y`"));
    }
}
