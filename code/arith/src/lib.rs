//! Arithmetic-expression evaluator for TAPL Chapters 3 and 4.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    True,
    False,
    If(Box<Term>, Box<Term>, Box<Term>),
    Zero,
    Succ(Box<Term>),
    Pred(Box<Term>),
    IsZero(Box<Term>),
}

impl Term {
    #[must_use]
    pub fn is_numeric_value(&self) -> bool {
        match self {
            Self::Zero => true,
            Self::Succ(inner) => inner.is_numeric_value(),
            _ => false,
        }
    }

    #[must_use]
    pub fn is_value(&self) -> bool {
        matches!(self, Self::True | Self::False) || self.is_numeric_value()
    }
}

#[must_use]
pub fn step(term: &Term) -> Option<Term> {
    match term {
        Term::If(guard, then_term, _) if **guard == Term::True => Some((**then_term).clone()),
        Term::If(guard, _, else_term) if **guard == Term::False => Some((**else_term).clone()),
        Term::If(guard, then_term, else_term) => step(guard).map(|next| {
            Term::If(
                Box::new(next),
                Box::new((**then_term).clone()),
                Box::new((**else_term).clone()),
            )
        }),
        Term::Succ(inner) => step(inner).map(|next| Term::Succ(Box::new(next))),
        Term::Pred(inner) if **inner == Term::Zero => Some(Term::Zero),
        Term::Pred(inner) => {
            if let Term::Succ(numeric) = inner.as_ref()
                && numeric.is_numeric_value()
            {
                return Some((**numeric).clone());
            }
            step(inner).map(|next| Term::Pred(Box::new(next)))
        }
        Term::IsZero(inner) if **inner == Term::Zero => Some(Term::True),
        Term::IsZero(inner) => {
            if let Term::Succ(numeric) = inner.as_ref()
                && numeric.is_numeric_value()
            {
                return Some(Term::False);
            }
            step(inner).map(|next| Term::IsZero(Box::new(next)))
        }
        Term::True | Term::False | Term::Zero => None,
    }
}

#[must_use]
pub fn eval(mut term: Term) -> Term {
    while let Some(next) = step(&term) {
        term = next;
    }
    term
}

/// Big-step evaluator corresponding to Exercise 3.5.17.
///
/// `None` means that the term is stuck rather than evaluating to a value.
#[must_use]
pub fn eval_big(term: &Term) -> Option<Term> {
    match term {
        Term::True | Term::False | Term::Zero => Some(term.clone()),
        Term::Succ(inner) => {
            let value = eval_big(inner)?;
            value
                .is_numeric_value()
                .then(|| Term::Succ(Box::new(value)))
        }
        Term::If(guard, then_term, else_term) => match eval_big(guard)? {
            Term::True => eval_big(then_term),
            Term::False => eval_big(else_term),
            _ => None,
        },
        Term::Pred(inner) => match eval_big(inner)? {
            Term::Zero => Some(Term::Zero),
            Term::Succ(numeric) if numeric.is_numeric_value() => Some(*numeric),
            _ => None,
        },
        Term::IsZero(inner) => match eval_big(inner)? {
            Term::Zero => Some(Term::True),
            Term::Succ(numeric) if numeric.is_numeric_value() => Some(Term::False),
            _ => None,
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Token {
    True,
    False,
    If,
    Then,
    Else,
    Zero,
    Succ,
    Pred,
    IsZero,
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
            b';' => {
                offset += 1;
                Token::Semicolon
            }
            byte if byte.is_ascii_alphabetic() => {
                offset += 1;
                while offset < bytes.len()
                    && (bytes[offset].is_ascii_alphanumeric() || bytes[offset] == b'_')
                {
                    offset += 1;
                }
                match &source[token_offset..offset] {
                    "true" => Token::True,
                    "false" => Token::False,
                    "if" => Token::If,
                    "then" => Token::Then,
                    "else" => Token::Else,
                    "succ" => Token::Succ,
                    "pred" => Token::Pred,
                    "iszero" => Token::IsZero,
                    word => {
                        return Err(ParseError {
                            offset: token_offset,
                            message: format!("unexpected identifier `{word}`"),
                        });
                    }
                }
            }
            b'0' => {
                offset += 1;
                Token::Zero
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

struct Parser {
    tokens: Vec<(Token, usize)>,
    cursor: usize,
    source_len: usize,
}

impl Parser {
    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.cursor).map(|(token, _)| token)
    }

    fn offset(&self) -> usize {
        self.tokens
            .get(self.cursor)
            .map_or(self.source_len, |(_, offset)| *offset)
    }

    fn consume(&mut self, expected: Token) -> Result<(), ParseError> {
        if self.current() == Some(&expected) {
            self.cursor += 1;
            Ok(())
        } else {
            Err(ParseError {
                offset: self.offset(),
                message: format!("expected {expected:?}, found {:?}", self.current()),
            })
        }
    }

    fn term(&mut self) -> Result<Term, ParseError> {
        match self.current() {
            Some(Token::True) => {
                self.cursor += 1;
                Ok(Term::True)
            }
            Some(Token::False) => {
                self.cursor += 1;
                Ok(Term::False)
            }
            Some(Token::Zero) => {
                self.cursor += 1;
                Ok(Term::Zero)
            }
            Some(Token::Succ) => {
                self.cursor += 1;
                Ok(Term::Succ(Box::new(self.term()?)))
            }
            Some(Token::Pred) => {
                self.cursor += 1;
                Ok(Term::Pred(Box::new(self.term()?)))
            }
            Some(Token::IsZero) => {
                self.cursor += 1;
                Ok(Term::IsZero(Box::new(self.term()?)))
            }
            Some(Token::If) => {
                self.cursor += 1;
                let guard = self.term()?;
                self.consume(Token::Then)?;
                let then_term = self.term()?;
                self.consume(Token::Else)?;
                let else_term = self.term()?;
                Ok(Term::If(
                    Box::new(guard),
                    Box::new(then_term),
                    Box::new(else_term),
                ))
            }
            Some(Token::LeftParen) => {
                self.cursor += 1;
                let term = self.term()?;
                self.consume(Token::RightParen)?;
                Ok(term)
            }
            token => Err(ParseError {
                offset: self.offset(),
                message: format!("expected a term, found {token:?}"),
            }),
        }
    }
}

/// Parse a semicolon-terminated sequence of arithmetic terms.
///
/// # Errors
///
/// Returns a [`ParseError`] when the input contains an unknown token, a
/// malformed term, or a missing terminating semicolon.
pub fn parse_program(source: &str) -> Result<Vec<Term>, ParseError> {
    let mut parser = Parser {
        tokens: lex(source)?,
        cursor: 0,
        source_len: source.len(),
    };
    let mut terms = Vec::new();
    while parser.current().is_some() {
        terms.push(parser.term()?);
        parser.consume(Token::Semicolon)?;
    }
    Ok(terms)
}

impl fmt::Display for Term {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::True => formatter.write_str("true"),
            Self::False => formatter.write_str("false"),
            Self::Zero => formatter.write_str("0"),
            Self::If(guard, then_term, else_term) => {
                write!(formatter, "if {guard} then {then_term} else {else_term}")
            }
            Self::Succ(inner) => write!(formatter, "succ ({inner})"),
            Self::Pred(inner) => write!(formatter, "pred ({inner})"),
            Self::IsZero(inner) => write!(formatter, "iszero ({inner})"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(term: Term) -> Box<Term> {
        Box::new(term)
    }

    #[test]
    fn official_examples_parse_and_evaluate() {
        let source = include_str!("../../../source/official-code/extracted/arith/test.f");
        let terms = parse_program(source).expect("official test file should parse");
        assert_eq!(terms.len(), 5);
        let results: Vec<_> = terms.into_iter().map(eval).collect();
        assert_eq!(
            results,
            vec![
                Term::True,
                Term::False,
                Term::Zero,
                Term::Succ(Box::new(Term::Zero)),
                Term::False,
            ]
        );
    }

    #[test]
    fn small_step_follows_the_congruence_rules() {
        let term = Term::Pred(boxed(Term::Succ(boxed(Term::Pred(boxed(Term::Zero))))));
        assert_eq!(
            step(&term),
            Some(Term::Pred(boxed(Term::Succ(boxed(Term::Zero)))))
        );
        assert_eq!(eval(term), Term::Zero);
    }

    #[test]
    fn big_step_agrees_with_small_step_on_values_and_stuck_terms() {
        let term = Term::If(
            boxed(Term::IsZero(boxed(Term::Pred(boxed(Term::Succ(boxed(
                Term::Zero,
            ))))))),
            boxed(Term::Succ(boxed(Term::Zero))),
            boxed(Term::False),
        );
        let small = eval(term.clone());
        assert_eq!(eval_big(&term), Some(small));

        let stuck = Term::Pred(boxed(Term::True));
        assert_eq!(eval(stuck.clone()), stuck);
        assert_eq!(eval_big(&stuck), None);
    }

    #[test]
    fn malformed_input_reports_an_offset() {
        let error = parse_program("if true then 0;").expect_err("else branch is required");
        assert!(error.offset > 0);
        assert!(error.message.contains("Else"));
    }
}
