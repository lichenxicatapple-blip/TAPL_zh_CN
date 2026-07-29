//! Executable companion for TAPL Chapters 8 through 14.
//!
//! The interpreter deliberately uses named variables and environments rather
//! than reproducing the OCaml checkers line for line.  Its types and behavior
//! cover typed arithmetic, the simply typed lambda-calculus, the principal
//! Chapter 11 extensions, references, and both forms of exceptions.

#![allow(clippy::large_enum_variant, clippy::module_name_repetitions)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Bottom,
    Base(String),
    Bool,
    Nat,
    Unit,
    Arrow(Box<Type>, Box<Type>),
    Product(Box<Type>, Box<Type>),
    Record(Vec<(String, Type)>),
    Variant(Vec<(String, Type)>),
    List(Box<Type>),
    Ref(Box<Type>),
    Exn,
}

impl Type {
    #[must_use]
    pub fn arrow(parameter: Self, result: Self) -> Self {
        Self::Arrow(Box::new(parameter), Box::new(result))
    }

    #[must_use]
    pub fn reference(contents: Self) -> Self {
        Self::Ref(Box::new(contents))
    }

    #[must_use]
    pub fn list(element: Self) -> Self {
        Self::List(Box::new(element))
    }
}

impl fmt::Display for Type {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bottom => formatter.write_str("Bot"),
            Self::Base(name) => formatter.write_str(name),
            Self::Bool => formatter.write_str("Bool"),
            Self::Nat => formatter.write_str("Nat"),
            Self::Unit => formatter.write_str("Unit"),
            Self::Arrow(parameter, result) => write!(formatter, "({parameter}->{result})"),
            Self::Product(left, right) => write!(formatter, "({left}*{right})"),
            Self::Record(fields) => {
                formatter.write_str("{")?;
                for (index, (label, field_type)) in fields.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(",")?;
                    }
                    write!(formatter, "{label}:{field_type}")?;
                }
                formatter.write_str("}")
            }
            Self::Variant(fields) => {
                formatter.write_str("<")?;
                for (index, (label, field_type)) in fields.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(",")?;
                    }
                    write!(formatter, "{label}:{field_type}")?;
                }
                formatter.write_str(">")
            }
            Self::List(element) => write!(formatter, "List {element}"),
            Self::Ref(contents) => write!(formatter, "Ref {contents}"),
            Self::Exn => formatter.write_str("Exn"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    Var(String),
    Abs {
        parameter: String,
        parameter_type: Type,
        body: Box<Term>,
    },
    App(Box<Term>, Box<Term>),
    True,
    False,
    If(Box<Term>, Box<Term>, Box<Term>),
    Nat(u64),
    Succ(Box<Term>),
    Pred(Box<Term>),
    IsZero(Box<Term>),
    Unit,
    Seq(Box<Term>, Box<Term>),
    Let {
        name: String,
        bound: Box<Term>,
        body: Box<Term>,
    },
    Ascribe(Box<Term>, Type),
    Pair(Box<Term>, Box<Term>),
    First(Box<Term>),
    Second(Box<Term>),
    Record(Vec<(String, Term)>),
    Project(Box<Term>, String),
    Variant {
        label: String,
        value: Box<Term>,
        variant_type: Type,
    },
    Case {
        scrutinee: Box<Term>,
        branches: Vec<VariantBranch>,
    },
    Fix(Box<Term>),
    Nil(Type),
    Cons {
        element_type: Type,
        head: Box<Term>,
        tail: Box<Term>,
    },
    IsNil(Box<Term>),
    Head(Box<Term>),
    Tail(Box<Term>),
    Ref(Box<Term>),
    Deref(Box<Term>),
    Assign(Box<Term>, Box<Term>),
    Location(usize),
    Error,
    Try {
        body: Box<Term>,
        handler: Box<Term>,
    },
    Exception(Box<Term>),
    Raise {
        value: Box<Term>,
        result_type: Type,
    },
    TryWith {
        body: Box<Term>,
        handler: Box<Term>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VariantBranch {
    pub label: String,
    pub binder: String,
    pub body: Term,
}

pub type TypeContext = BTreeMap<String, Type>;
pub type StoreTyping = Vec<Type>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeError {
    UnknownVariable(String),
    UnknownLocation(usize),
    Expected { expected: Type, actual: Type },
    ExpectedFunction(Type),
    ExpectedProduct(Type),
    ExpectedRecord(Type),
    ExpectedVariant(Type),
    ExpectedList(Type),
    ExpectedReference(Type),
    BranchMismatch { left: Type, right: Type },
    MissingLabel(String),
    DuplicateLabel(String),
    IncompleteCase,
}

impl fmt::Display for TypeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownVariable(name) => write!(formatter, "unbound variable `{name}`"),
            Self::UnknownLocation(location) => {
                write!(
                    formatter,
                    "location {location} is absent from the store typing"
                )
            }
            Self::Expected { expected, actual } => {
                write!(formatter, "expected {expected}, found {actual}")
            }
            Self::ExpectedFunction(actual) => {
                write!(formatter, "expected an arrow, found {actual}")
            }
            Self::ExpectedProduct(actual) => {
                write!(formatter, "expected a product, found {actual}")
            }
            Self::ExpectedRecord(actual) => write!(formatter, "expected a record, found {actual}"),
            Self::ExpectedVariant(actual) => {
                write!(formatter, "expected a variant, found {actual}")
            }
            Self::ExpectedList(actual) => write!(formatter, "expected a list, found {actual}"),
            Self::ExpectedReference(actual) => {
                write!(formatter, "expected a reference, found {actual}")
            }
            Self::BranchMismatch { left, right } => {
                write!(formatter, "branches have types {left} and {right}")
            }
            Self::MissingLabel(label) => write!(formatter, "type has no label `{label}`"),
            Self::DuplicateLabel(label) => write!(formatter, "duplicate label `{label}`"),
            Self::IncompleteCase => formatter.write_str("case does not cover the complete variant"),
        }
    }
}

impl std::error::Error for TypeError {}

fn subtype(left: &Type, right: &Type) -> bool {
    left == right || matches!(left, Type::Bottom)
}

fn expect_type(actual: &Type, expected: &Type) -> Result<(), TypeError> {
    if subtype(actual, expected) {
        Ok(())
    } else {
        Err(TypeError::Expected {
            expected: expected.clone(),
            actual: actual.clone(),
        })
    }
}

fn join(left: Type, right: Type) -> Result<Type, TypeError> {
    if subtype(&left, &right) {
        Ok(right)
    } else if subtype(&right, &left) {
        Ok(left)
    } else {
        Err(TypeError::BranchMismatch { left, right })
    }
}

fn labels_are_unique<T>(fields: &[(String, T)]) -> Result<(), TypeError> {
    let mut labels = BTreeSet::new();
    for (label, _) in fields {
        if !labels.insert(label) {
            return Err(TypeError::DuplicateLabel(label.clone()));
        }
    }
    Ok(())
}

/// Calculate the type of `term` under the supplied variable and store typings.
///
/// # Errors
///
/// Returns [`TypeError`] when the term is not derivable by the implemented
/// typing rules.
#[allow(clippy::too_many_lines)]
pub fn type_of(
    term: &Term,
    context: &TypeContext,
    store_typing: &StoreTyping,
) -> Result<Type, TypeError> {
    match term {
        Term::Var(name) => context
            .get(name)
            .cloned()
            .ok_or_else(|| TypeError::UnknownVariable(name.clone())),
        Term::Abs {
            parameter,
            parameter_type,
            body,
        } => {
            let mut extended = context.clone();
            extended.insert(parameter.clone(), parameter_type.clone());
            Ok(Type::arrow(
                parameter_type.clone(),
                type_of(body, &extended, store_typing)?,
            ))
        }
        Term::App(function, argument) => {
            let function_type = type_of(function, context, store_typing)?;
            let argument_type = type_of(argument, context, store_typing)?;
            match function_type {
                Type::Arrow(parameter, result) => {
                    expect_type(&argument_type, &parameter)?;
                    Ok(*result)
                }
                Type::Bottom => Ok(Type::Bottom),
                actual => Err(TypeError::ExpectedFunction(actual)),
            }
        }
        Term::True | Term::False => Ok(Type::Bool),
        Term::If(guard, then_branch, else_branch) => {
            let guard_type = type_of(guard, context, store_typing)?;
            expect_type(&guard_type, &Type::Bool)?;
            join(
                type_of(then_branch, context, store_typing)?,
                type_of(else_branch, context, store_typing)?,
            )
        }
        Term::Nat(_) => Ok(Type::Nat),
        Term::Succ(argument) | Term::Pred(argument) => {
            let argument_type = type_of(argument, context, store_typing)?;
            expect_type(&argument_type, &Type::Nat)?;
            Ok(Type::Nat)
        }
        Term::IsZero(argument) => {
            let argument_type = type_of(argument, context, store_typing)?;
            expect_type(&argument_type, &Type::Nat)?;
            Ok(Type::Bool)
        }
        Term::Unit => Ok(Type::Unit),
        Term::Seq(first, second) => {
            let first_type = type_of(first, context, store_typing)?;
            expect_type(&first_type, &Type::Unit)?;
            type_of(second, context, store_typing)
        }
        Term::Let { name, bound, body } => {
            let bound_type = type_of(bound, context, store_typing)?;
            let mut extended = context.clone();
            extended.insert(name.clone(), bound_type);
            type_of(body, &extended, store_typing)
        }
        Term::Ascribe(inner, ascribed) => {
            let actual = type_of(inner, context, store_typing)?;
            expect_type(&actual, ascribed)?;
            Ok(ascribed.clone())
        }
        Term::Pair(left, right) => Ok(Type::Product(
            Box::new(type_of(left, context, store_typing)?),
            Box::new(type_of(right, context, store_typing)?),
        )),
        Term::First(pair) => match type_of(pair, context, store_typing)? {
            Type::Product(left, _) => Ok(*left),
            actual => Err(TypeError::ExpectedProduct(actual)),
        },
        Term::Second(pair) => match type_of(pair, context, store_typing)? {
            Type::Product(_, right) => Ok(*right),
            actual => Err(TypeError::ExpectedProduct(actual)),
        },
        Term::Record(fields) => {
            labels_are_unique(fields)?;
            fields
                .iter()
                .map(|(label, field)| Ok((label.clone(), type_of(field, context, store_typing)?)))
                .collect::<Result<Vec<_>, _>>()
                .map(Type::Record)
        }
        Term::Project(record, label) => match type_of(record, context, store_typing)? {
            Type::Record(fields) => fields
                .into_iter()
                .find_map(|(candidate, field_type)| (candidate == *label).then_some(field_type))
                .ok_or_else(|| TypeError::MissingLabel(label.clone())),
            actual => Err(TypeError::ExpectedRecord(actual)),
        },
        Term::Variant {
            label,
            value,
            variant_type,
        } => {
            let Type::Variant(fields) = variant_type else {
                return Err(TypeError::ExpectedVariant(variant_type.clone()));
            };
            labels_are_unique(fields)?;
            let expected = fields
                .iter()
                .find_map(|(candidate, field_type)| {
                    (candidate == label).then_some(field_type.clone())
                })
                .ok_or_else(|| TypeError::MissingLabel(label.clone()))?;
            let actual = type_of(value, context, store_typing)?;
            expect_type(&actual, &expected)?;
            Ok(variant_type.clone())
        }
        Term::Case {
            scrutinee,
            branches,
        } => {
            let Type::Variant(fields) = type_of(scrutinee, context, store_typing)? else {
                return Err(TypeError::ExpectedVariant(type_of(
                    scrutinee,
                    context,
                    store_typing,
                )?));
            };
            if fields.len() != branches.len() {
                return Err(TypeError::IncompleteCase);
            }
            let mut result = None;
            for (label, field_type) in fields {
                let branch = branches
                    .iter()
                    .find(|branch| branch.label == label)
                    .ok_or(TypeError::IncompleteCase)?;
                let mut extended = context.clone();
                extended.insert(branch.binder.clone(), field_type);
                let branch_type = type_of(&branch.body, &extended, store_typing)?;
                result = Some(match result {
                    None => branch_type,
                    Some(previous) => join(previous, branch_type)?,
                });
            }
            result.ok_or(TypeError::IncompleteCase)
        }
        Term::Fix(function) => match type_of(function, context, store_typing)? {
            Type::Arrow(parameter, result) if subtype(&result, &parameter) => Ok(*parameter),
            actual => Err(TypeError::ExpectedFunction(actual)),
        },
        Term::Nil(element_type) => Ok(Type::list(element_type.clone())),
        Term::Cons {
            element_type,
            head,
            tail,
        } => {
            let head_type = type_of(head, context, store_typing)?;
            expect_type(&head_type, element_type)?;
            let tail_type = type_of(tail, context, store_typing)?;
            let list_type = Type::list(element_type.clone());
            expect_type(&tail_type, &list_type)?;
            Ok(list_type)
        }
        Term::IsNil(list) => {
            if matches!(type_of(list, context, store_typing)?, Type::List(_)) {
                Ok(Type::Bool)
            } else {
                Err(TypeError::ExpectedList(type_of(
                    list,
                    context,
                    store_typing,
                )?))
            }
        }
        Term::Head(list) => match type_of(list, context, store_typing)? {
            Type::List(element) => Ok(*element),
            actual => Err(TypeError::ExpectedList(actual)),
        },
        Term::Tail(list) => match type_of(list, context, store_typing)? {
            Type::List(element) => Ok(Type::List(element)),
            actual => Err(TypeError::ExpectedList(actual)),
        },
        Term::Ref(initial) => Ok(Type::reference(type_of(initial, context, store_typing)?)),
        Term::Deref(reference) => match type_of(reference, context, store_typing)? {
            Type::Ref(contents) => Ok(*contents),
            actual => Err(TypeError::ExpectedReference(actual)),
        },
        Term::Assign(reference, value) => match type_of(reference, context, store_typing)? {
            Type::Ref(contents) => {
                let actual = type_of(value, context, store_typing)?;
                expect_type(&actual, &contents)?;
                Ok(Type::Unit)
            }
            actual => Err(TypeError::ExpectedReference(actual)),
        },
        Term::Location(location) => store_typing
            .get(*location)
            .cloned()
            .map(Type::reference)
            .ok_or(TypeError::UnknownLocation(*location)),
        Term::Error => Ok(Type::Bottom),
        Term::Try { body, handler } => join(
            type_of(body, context, store_typing)?,
            type_of(handler, context, store_typing)?,
        ),
        Term::Exception(_) => Ok(Type::Exn),
        Term::Raise { value, result_type } => {
            let payload_type = type_of(value, context, store_typing)?;
            expect_type(&payload_type, &Type::Exn)?;
            Ok(result_type.clone())
        }
        Term::TryWith { body, handler } => {
            let body_type = type_of(body, context, store_typing)?;
            let expected_handler = Type::arrow(Type::Exn, body_type.clone());
            let handler_type = type_of(handler, context, store_typing)?;
            expect_type(&handler_type, &expected_handler)?;
            Ok(body_type)
        }
    }
}

type Environment = BTreeMap<String, Value>;

/// An arbitrary-precision natural number stored in base \(10^9\).
///
/// The empty limb vector represents zero.  This keeps the host
/// implementation faithful to TAPL's unbounded chains of `succ`, including
/// values just beyond `u64::MAX`, without adding a third-party dependency.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Natural(Vec<u32>);

impl Natural {
    const BASE: u64 = 1_000_000_000;
    const MAX_LIMB: u32 = 999_999_999;

    fn from_u64(mut value: u64) -> Self {
        let mut limbs = Vec::new();
        while value > 0 {
            limbs.push((value % Self::BASE) as u32);
            value /= Self::BASE;
        }
        Self(limbs)
    }

    fn increment(mut self) -> Self {
        let mut index = 0;
        loop {
            if index == self.0.len() {
                self.0.push(1);
                break;
            }
            if u64::from(self.0[index]) + 1 < Self::BASE {
                self.0[index] += 1;
                break;
            }
            self.0[index] = 0;
            index += 1;
        }
        self
    }

    fn predecessor(mut self) -> Self {
        if self.0.is_empty() {
            return self;
        }
        let mut index = 0;
        loop {
            if self.0[index] > 0 {
                self.0[index] -= 1;
                break;
            }
            self.0[index] = Self::MAX_LIMB;
            index += 1;
        }
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
        self
    }

    fn is_zero(&self) -> bool {
        self.0.is_empty()
    }

    fn to_u64(&self) -> Option<u64> {
        self.0.iter().rev().try_fold(0_u64, |accumulator, limb| {
            accumulator
                .checked_mul(Self::BASE)?
                .checked_add(u64::from(*limb))
        })
    }
}

impl fmt::Display for Natural {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Some((last, remaining)) = self.0.split_last() else {
            return formatter.write_str("0");
        };
        write!(formatter, "{last}")?;
        for limb in remaining.iter().rev() {
            write!(formatter, "{limb:09}")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
pub enum Value {
    Bool(bool),
    Nat(Natural),
    Unit,
    Closure {
        parameter: String,
        body: Term,
        environment: Environment,
    },
    Fix(Box<Value>),
    Pair(Box<Value>, Box<Value>),
    Record(Vec<(String, Value)>),
    Variant {
        label: String,
        value: Box<Value>,
    },
    List(Vec<Value>),
    Location(usize),
}

impl Value {
    #[must_use]
    pub const fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_nat(&self) -> Option<u64> {
        match self {
            Self::Nat(value) => value.to_u64(),
            _ => None,
        }
    }

    fn into_natural(self) -> Option<Natural> {
        match self {
            Self::Nat(value) => Some(value),
            _ => None,
        }
    }

    #[must_use]
    pub const fn location(&self) -> Option<usize> {
        match self {
            Self::Location(location) => Some(*location),
            _ => None,
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool(value) => write!(formatter, "{value}"),
            Self::Nat(value) => write!(formatter, "{value}"),
            Self::Unit => formatter.write_str("unit"),
            Self::Closure { .. } | Self::Fix(_) => formatter.write_str("<fun>"),
            Self::Pair(left, right) => write!(formatter, "{{{left},{right}}}"),
            Self::Record(fields) => {
                formatter.write_str("{")?;
                for (index, (label, value)) in fields.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(",")?;
                    }
                    write!(formatter, "{label}={value}")?;
                }
                formatter.write_str("}")
            }
            Self::Variant { label, value } => write!(formatter, "<{label}={value}>"),
            Self::List(elements) => {
                formatter.write_str("[")?;
                for (index, value) in elements.iter().enumerate() {
                    if index > 0 {
                        formatter.write_str(",")?;
                    }
                    write!(formatter, "{value}")?;
                }
                formatter.write_str("]")
            }
            Self::Location(location) => write!(formatter, "<loc #{location}>"),
        }
    }
}

pub type Store = Vec<Value>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeError {
    Abort,
    UncaughtException(String),
    UnboundVariable(String),
    ExpectedBoolean,
    ExpectedNatural,
    ExpectedFunction,
    ExpectedPair,
    ExpectedRecord,
    ExpectedVariant,
    ExpectedList,
    EmptyList,
    ExpectedLocation,
    InvalidLocation(usize),
    StepLimit,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Abort => formatter.write_str("evaluation aborted with error"),
            Self::UncaughtException(value) => write!(formatter, "uncaught exception: {value}"),
            Self::UnboundVariable(name) => write!(formatter, "unbound variable `{name}`"),
            Self::ExpectedBoolean => formatter.write_str("expected a boolean"),
            Self::ExpectedNatural => formatter.write_str("expected a natural number"),
            Self::ExpectedFunction => formatter.write_str("expected a function"),
            Self::ExpectedPair => formatter.write_str("expected a pair"),
            Self::ExpectedRecord => formatter.write_str("expected a record"),
            Self::ExpectedVariant => formatter.write_str("expected a variant"),
            Self::ExpectedList => formatter.write_str("expected a list"),
            Self::EmptyList => formatter.write_str("empty list has no head or tail"),
            Self::ExpectedLocation => formatter.write_str("expected a store location"),
            Self::InvalidLocation(location) => write!(formatter, "invalid location {location}"),
            Self::StepLimit => formatter.write_str("evaluation step limit exhausted"),
        }
    }
}

impl std::error::Error for RuntimeError {}

enum Signal {
    Runtime(RuntimeError),
    Abort,
    Raise(Value),
}

impl From<RuntimeError> for Signal {
    fn from(error: RuntimeError) -> Self {
        Self::Runtime(error)
    }
}

struct Evaluator<'store> {
    store: &'store mut Store,
    remaining: usize,
}

impl Evaluator<'_> {
    fn tick(&mut self) -> Result<(), Signal> {
        self.remaining = self
            .remaining
            .checked_sub(1)
            .ok_or(Signal::Runtime(RuntimeError::StepLimit))?;
        Ok(())
    }

    fn apply(&mut self, function: Value, argument: Value) -> Result<Value, Signal> {
        self.tick()?;
        match function {
            Value::Closure {
                parameter,
                body,
                mut environment,
            } => {
                environment.insert(parameter, argument);
                self.eval(&body, &environment)
            }
            Value::Fix(function) => {
                let unfolded = self.fixed_point((*function).clone())?;
                self.apply(unfolded, argument)
            }
            _ => Err(RuntimeError::ExpectedFunction.into()),
        }
    }

    fn fixed_point(&mut self, function: Value) -> Result<Value, Signal> {
        let unfolded = self.apply(function.clone(), Value::Fix(Box::new(function.clone())))?;
        if matches!(unfolded, Value::Fix(_)) {
            self.fixed_point(function)
        } else {
            Ok(unfolded)
        }
    }

    #[allow(clippy::too_many_lines)]
    fn eval(&mut self, term: &Term, environment: &Environment) -> Result<Value, Signal> {
        self.tick()?;
        match term {
            Term::Var(name) => environment
                .get(name)
                .cloned()
                .ok_or_else(|| RuntimeError::UnboundVariable(name.clone()).into()),
            Term::Abs {
                parameter, body, ..
            } => Ok(Value::Closure {
                parameter: parameter.clone(),
                body: body.as_ref().clone(),
                environment: environment.clone(),
            }),
            Term::App(function, argument) => {
                let function = self.eval(function, environment)?;
                let argument = self.eval(argument, environment)?;
                self.apply(function, argument)
            }
            Term::True => Ok(Value::Bool(true)),
            Term::False => Ok(Value::Bool(false)),
            Term::If(guard, then_branch, else_branch) => {
                if self
                    .eval(guard, environment)?
                    .as_bool()
                    .ok_or(RuntimeError::ExpectedBoolean)?
                {
                    self.eval(then_branch, environment)
                } else {
                    self.eval(else_branch, environment)
                }
            }
            Term::Nat(value) => Ok(Value::Nat(Natural::from_u64(*value))),
            Term::Succ(argument) => {
                let natural = self
                    .eval(argument, environment)?
                    .into_natural()
                    .ok_or(RuntimeError::ExpectedNatural)?;
                Ok(Value::Nat(natural.increment()))
            }
            Term::Pred(argument) => {
                let natural = self
                    .eval(argument, environment)?
                    .into_natural()
                    .ok_or(RuntimeError::ExpectedNatural)?;
                Ok(Value::Nat(natural.predecessor()))
            }
            Term::IsZero(argument) => {
                let natural = self
                    .eval(argument, environment)?
                    .into_natural()
                    .ok_or(RuntimeError::ExpectedNatural)?;
                Ok(Value::Bool(natural.is_zero()))
            }
            Term::Unit => Ok(Value::Unit),
            Term::Seq(first, second) => {
                self.eval(first, environment)?;
                self.eval(second, environment)
            }
            Term::Let { name, bound, body } => {
                let value = self.eval(bound, environment)?;
                let mut extended = environment.clone();
                extended.insert(name.clone(), value);
                self.eval(body, &extended)
            }
            Term::Ascribe(inner, _) => self.eval(inner, environment),
            Term::Pair(left, right) => Ok(Value::Pair(
                Box::new(self.eval(left, environment)?),
                Box::new(self.eval(right, environment)?),
            )),
            Term::First(pair) => match self.eval(pair, environment)? {
                Value::Pair(left, _) => Ok(*left),
                _ => Err(RuntimeError::ExpectedPair.into()),
            },
            Term::Second(pair) => match self.eval(pair, environment)? {
                Value::Pair(_, right) => Ok(*right),
                _ => Err(RuntimeError::ExpectedPair.into()),
            },
            Term::Record(fields) => fields
                .iter()
                .map(|(label, field)| Ok((label.clone(), self.eval(field, environment)?)))
                .collect::<Result<Vec<_>, Signal>>()
                .map(Value::Record),
            Term::Project(record, label) => match self.eval(record, environment)? {
                Value::Record(fields) => fields
                    .into_iter()
                    .find_map(|(candidate, value)| (candidate == *label).then_some(value))
                    .ok_or_else(|| RuntimeError::ExpectedRecord.into()),
                _ => Err(RuntimeError::ExpectedRecord.into()),
            },
            Term::Variant { label, value, .. } => Ok(Value::Variant {
                label: label.clone(),
                value: Box::new(self.eval(value, environment)?),
            }),
            Term::Case {
                scrutinee,
                branches,
            } => {
                let Value::Variant { label, value } = self.eval(scrutinee, environment)? else {
                    return Err(RuntimeError::ExpectedVariant.into());
                };
                let branch = branches
                    .iter()
                    .find(|branch| branch.label == label)
                    .ok_or(RuntimeError::ExpectedVariant)?;
                let mut extended = environment.clone();
                extended.insert(branch.binder.clone(), *value);
                self.eval(&branch.body, &extended)
            }
            Term::Fix(function) => {
                let function = self.eval(function, environment)?;
                self.fixed_point(function)
            }
            Term::Nil(_) => Ok(Value::List(Vec::new())),
            Term::Cons { head, tail, .. } => {
                let head = self.eval(head, environment)?;
                let Value::List(mut tail) = self.eval(tail, environment)? else {
                    return Err(RuntimeError::ExpectedList.into());
                };
                tail.insert(0, head);
                Ok(Value::List(tail))
            }
            Term::IsNil(list) => match self.eval(list, environment)? {
                Value::List(elements) => Ok(Value::Bool(elements.is_empty())),
                _ => Err(RuntimeError::ExpectedList.into()),
            },
            Term::Head(list) => match self.eval(list, environment)? {
                Value::List(elements) => elements
                    .into_iter()
                    .next()
                    .ok_or_else(|| RuntimeError::EmptyList.into()),
                _ => Err(RuntimeError::ExpectedList.into()),
            },
            Term::Tail(list) => match self.eval(list, environment)? {
                Value::List(mut elements) if !elements.is_empty() => {
                    elements.remove(0);
                    Ok(Value::List(elements))
                }
                Value::List(_) => Err(RuntimeError::EmptyList.into()),
                _ => Err(RuntimeError::ExpectedList.into()),
            },
            Term::Ref(initial) => {
                let initial = self.eval(initial, environment)?;
                let location = self.store.len();
                self.store.push(initial);
                Ok(Value::Location(location))
            }
            Term::Deref(reference) => {
                let location = self
                    .eval(reference, environment)?
                    .location()
                    .ok_or(RuntimeError::ExpectedLocation)?;
                self.store
                    .get(location)
                    .cloned()
                    .ok_or_else(|| RuntimeError::InvalidLocation(location).into())
            }
            Term::Assign(reference, value) => {
                let location = self
                    .eval(reference, environment)?
                    .location()
                    .ok_or(RuntimeError::ExpectedLocation)?;
                let value = self.eval(value, environment)?;
                let destination = self
                    .store
                    .get_mut(location)
                    .ok_or(RuntimeError::InvalidLocation(location))?;
                *destination = value;
                Ok(Value::Unit)
            }
            Term::Location(location) => {
                if *location < self.store.len() {
                    Ok(Value::Location(*location))
                } else {
                    Err(RuntimeError::InvalidLocation(*location).into())
                }
            }
            Term::Error => Err(Signal::Abort),
            Term::Try { body, handler } => match self.eval(body, environment) {
                Err(Signal::Abort) => self.eval(handler, environment),
                other => other,
            },
            Term::Exception(value) => self.eval(value, environment),
            Term::Raise { value, .. } => {
                let payload = self.eval(value, environment)?;
                Err(Signal::Raise(payload))
            }
            Term::TryWith { body, handler } => match self.eval(body, environment) {
                Err(Signal::Raise(payload)) => {
                    let handler = self.eval(handler, environment)?;
                    self.apply(handler, payload)
                }
                other => other,
            },
        }
    }
}

/// Evaluate a closed term with a configurable recursion limit.
///
/// # Errors
///
/// Returns [`RuntimeError`] for an unbound variable, an invalid run-time
/// operation, an uncaught exception, or an exhausted evaluation limit.
pub fn eval_with_limit(
    term: &Term,
    store: &mut Store,
    limit: usize,
) -> Result<Value, RuntimeError> {
    let result = Evaluator {
        store,
        remaining: limit,
    }
    .eval(term, &Environment::new());
    match result {
        Ok(value) => Ok(value),
        Err(Signal::Runtime(error)) => Err(error),
        Err(Signal::Abort) => Err(RuntimeError::Abort),
        Err(Signal::Raise(value)) => Err(RuntimeError::UncaughtException(value.to_string())),
    }
}

/// Evaluate a closed term with the default limit used by the examples.
///
/// # Errors
///
/// Returns [`RuntimeError`] under the same conditions as
/// [`eval_with_limit`].
pub fn eval(term: &Term, store: &mut Store) -> Result<Value, RuntimeError> {
    eval_with_limit(term, store, 100_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_context() -> TypeContext {
        TypeContext::new()
    }

    fn abs(parameter: &str, parameter_type: Type, body: Term) -> Term {
        Term::Abs {
            parameter: parameter.into(),
            parameter_type,
            body: Box::new(body),
        }
    }

    fn app(function: Term, argument: Term) -> Term {
        Term::App(Box::new(function), Box::new(argument))
    }

    #[test]
    fn typed_arithmetic_checks_and_evaluates() {
        let term = Term::IsZero(Box::new(Term::Pred(Box::new(Term::Succ(Box::new(
            Term::Nat(0),
        ))))));
        assert_eq!(
            type_of(&term, &empty_context(), &vec![]).unwrap(),
            Type::Bool
        );
        assert_eq!(eval(&term, &mut vec![]).unwrap().as_bool(), Some(true));
    }

    #[test]
    fn chapter_8_examples_have_the_expected_types() {
        let boolean_example = Term::If(
            Box::new(Term::True),
            Box::new(Term::False),
            Box::new(Term::True),
        );
        let natural_example = Term::Pred(Box::new(Term::Succ(Box::new(Term::Pred(Box::new(
            Term::Succ(Box::new(Term::Nat(0))),
        ))))));
        assert_eq!(
            type_of(&boolean_example, &empty_context(), &vec![]).unwrap(),
            Type::Bool
        );
        assert_eq!(
            type_of(&natural_example, &empty_context(), &vec![]).unwrap(),
            Type::Nat
        );

        let conservative_rejections = [
            Term::If(
                Box::new(Term::IsZero(Box::new(Term::Nat(0)))),
                Box::new(Term::Nat(0)),
                Box::new(Term::False),
            ),
            Term::If(
                Box::new(Term::True),
                Box::new(Term::Nat(0)),
                Box::new(Term::False),
            ),
        ];
        for term in conservative_rejections {
            assert!(type_of(&term, &empty_context(), &vec![]).is_err());
        }
    }

    #[test]
    fn arithmetic_operators_reject_arguments_of_the_wrong_type() {
        let ill_typed_terms = [
            Term::Succ(Box::new(Term::False)),
            Term::Pred(Box::new(Term::True)),
            Term::IsZero(Box::new(Term::False)),
        ];
        for term in ill_typed_terms {
            assert!(type_of(&term, &empty_context(), &vec![]).is_err());
        }
    }

    #[test]
    fn arithmetic_evaluation_covers_zero_and_successor_cases() {
        let cases = [
            (Term::Pred(Box::new(Term::Nat(0))), "0"),
            (
                Term::Pred(Box::new(Term::Succ(Box::new(Term::Nat(7))))),
                "7",
            ),
            (Term::IsZero(Box::new(Term::Nat(0))), "true"),
            (
                Term::IsZero(Box::new(Term::Succ(Box::new(Term::Nat(0))))),
                "false",
            ),
        ];
        for (term, expected) in cases {
            assert_eq!(eval(&term, &mut vec![]).unwrap().to_string(), expected);
        }
    }

    #[test]
    fn natural_successor_is_not_limited_by_u64() {
        let term = Term::Succ(Box::new(Term::Nat(u64::MAX)));
        let value = eval(&term, &mut vec![]).unwrap();
        assert_eq!(value.to_string(), "18446744073709551616");
        assert_eq!(value.as_nat(), None);
        assert_eq!(
            eval(&Term::Pred(Box::new(term)), &mut vec![])
                .unwrap()
                .as_nat(),
            Some(u64::MAX)
        );
    }

    #[test]
    fn ill_typed_guard_is_rejected() {
        let term = Term::If(
            Box::new(Term::Nat(0)),
            Box::new(Term::True),
            Box::new(Term::False),
        );
        assert!(type_of(&term, &empty_context(), &vec![]).is_err());
    }

    #[test]
    fn typed_identity_application() {
        let term = app(abs("x", Type::Bool, Term::Var("x".into())), Term::True);
        assert_eq!(
            type_of(&term, &empty_context(), &vec![]).unwrap(),
            Type::Bool
        );
        assert_eq!(eval(&term, &mut vec![]).unwrap().as_bool(), Some(true));
    }

    #[test]
    fn let_pair_and_projection() {
        let term = Term::Let {
            name: "p".into(),
            bound: Box::new(Term::Pair(Box::new(Term::Nat(3)), Box::new(Term::False))),
            body: Box::new(Term::First(Box::new(Term::Var("p".into())))),
        };
        assert_eq!(
            type_of(&term, &empty_context(), &vec![]).unwrap(),
            Type::Nat
        );
        assert_eq!(eval(&term, &mut vec![]).unwrap().as_nat(), Some(3));
    }

    #[test]
    fn records_project_by_label() {
        let term = Term::Project(
            Box::new(Term::Record(vec![
                ("partno".into(), Term::Nat(5524)),
                ("available".into(), Term::True),
            ])),
            "available".into(),
        );
        assert_eq!(
            type_of(&term, &empty_context(), &vec![]).unwrap(),
            Type::Bool
        );
        assert_eq!(eval(&term, &mut vec![]).unwrap().as_bool(), Some(true));
    }

    #[test]
    fn variants_select_the_matching_branch() {
        let option = Type::Variant(vec![
            ("none".into(), Type::Unit),
            ("some".into(), Type::Nat),
        ]);
        let term = Term::Case {
            scrutinee: Box::new(Term::Variant {
                label: "some".into(),
                value: Box::new(Term::Nat(5)),
                variant_type: option,
            }),
            branches: vec![
                VariantBranch {
                    label: "none".into(),
                    binder: "u".into(),
                    body: Term::Nat(999),
                },
                VariantBranch {
                    label: "some".into(),
                    binder: "n".into(),
                    body: Term::Var("n".into()),
                },
            ],
        };
        assert_eq!(
            type_of(&term, &empty_context(), &vec![]).unwrap(),
            Type::Nat
        );
        assert_eq!(eval(&term, &mut vec![]).unwrap().as_nat(), Some(5));
    }

    #[test]
    fn lists_support_cons_head_and_tail() {
        let list = Term::Cons {
            element_type: Type::Nat,
            head: Box::new(Term::Nat(1)),
            tail: Box::new(Term::Cons {
                element_type: Type::Nat,
                head: Box::new(Term::Nat(2)),
                tail: Box::new(Term::Nil(Type::Nat)),
            }),
        };
        let term = Term::Head(Box::new(Term::Tail(Box::new(list))));
        assert_eq!(
            type_of(&term, &empty_context(), &vec![]).unwrap(),
            Type::Nat
        );
        assert_eq!(eval(&term, &mut vec![]).unwrap().as_nat(), Some(2));
    }

    #[test]
    fn fix_builds_an_evenness_function() {
        let nat_to_bool = Type::arrow(Type::Nat, Type::Bool);
        let generator = abs(
            "ie",
            nat_to_bool.clone(),
            abs(
                "x",
                Type::Nat,
                Term::If(
                    Box::new(Term::IsZero(Box::new(Term::Var("x".into())))),
                    Box::new(Term::True),
                    Box::new(Term::If(
                        Box::new(Term::IsZero(Box::new(Term::Pred(Box::new(Term::Var(
                            "x".into(),
                        )))))),
                        Box::new(Term::False),
                        Box::new(app(
                            Term::Var("ie".into()),
                            Term::Pred(Box::new(Term::Pred(Box::new(Term::Var("x".into()))))),
                        )),
                    )),
                ),
            ),
        );
        let term = app(Term::Fix(Box::new(generator)), Term::Nat(8));
        assert_eq!(
            type_of(&term, &empty_context(), &vec![]).unwrap(),
            Type::Bool
        );
        assert_eq!(eval(&term, &mut vec![]).unwrap().as_bool(), Some(true));
    }

    #[test]
    fn references_allocate_assign_and_dereference() {
        let term = Term::Let {
            name: "r".into(),
            bound: Box::new(Term::Ref(Box::new(Term::Nat(5)))),
            body: Box::new(Term::Seq(
                Box::new(Term::Assign(
                    Box::new(Term::Var("r".into())),
                    Box::new(Term::Nat(7)),
                )),
                Box::new(Term::Deref(Box::new(Term::Var("r".into())))),
            )),
        };
        assert_eq!(
            type_of(&term, &empty_context(), &vec![]).unwrap(),
            Type::Nat
        );
        let mut store = Vec::new();
        assert_eq!(eval(&term, &mut store).unwrap().as_nat(), Some(7));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn aliases_share_one_store_cell() {
        let term = Term::Let {
            name: "r".into(),
            bound: Box::new(Term::Ref(Box::new(Term::Nat(5)))),
            body: Box::new(Term::Let {
                name: "s".into(),
                bound: Box::new(Term::Var("r".into())),
                body: Box::new(Term::Seq(
                    Box::new(Term::Assign(
                        Box::new(Term::Var("s".into())),
                        Box::new(Term::Nat(82)),
                    )),
                    Box::new(Term::Deref(Box::new(Term::Var("r".into())))),
                )),
            }),
        };
        assert_eq!(eval(&term, &mut vec![]).unwrap().as_nat(), Some(82));
    }

    #[test]
    fn error_can_be_handled() {
        let term = Term::Try {
            body: Box::new(Term::Error),
            handler: Box::new(Term::Nat(42)),
        };
        assert_eq!(
            type_of(&term, &empty_context(), &vec![]).unwrap(),
            Type::Nat
        );
        assert_eq!(eval(&term, &mut vec![]).unwrap().as_nat(), Some(42));
    }

    #[test]
    fn unhandled_error_aborts() {
        assert_eq!(
            eval(&Term::Error, &mut vec![]).unwrap_err(),
            RuntimeError::Abort
        );
    }

    #[test]
    fn raised_value_is_passed_to_handler() {
        let term = Term::TryWith {
            body: Box::new(Term::Raise {
                value: Box::new(Term::Exception(Box::new(Term::Nat(7)))),
                result_type: Type::Nat,
            }),
            handler: Box::new(abs("e", Type::Exn, Term::Nat(99))),
        };
        assert_eq!(
            type_of(&term, &empty_context(), &vec![]).unwrap(),
            Type::Nat
        );
        assert_eq!(eval(&term, &mut vec![]).unwrap().as_nat(), Some(99));
    }

    #[test]
    fn ordinary_error_propagates_from_both_application_positions() {
        let divergent = Term::Fix(Box::new(abs("x", Type::Nat, Term::Var("x".into()))));
        let left_error = app(Term::Error, divergent);
        let right_error = app(abs("x", Type::Nat, Term::Var("x".into())), Term::Error);
        assert_eq!(
            eval_with_limit(&left_error, &mut vec![], 30).unwrap_err(),
            RuntimeError::Abort
        );
        assert_eq!(
            eval(&right_error, &mut vec![]).unwrap_err(),
            RuntimeError::Abort
        );
    }

    #[test]
    fn raised_values_propagate_left_to_right_through_application() {
        let raise = |payload| Term::Raise {
            value: Box::new(Term::Exception(Box::new(Term::Nat(payload)))),
            result_type: Type::Nat,
        };
        let left_raise = app(raise(1), raise(2));
        let right_raise = app(abs("x", Type::Nat, Term::Var("x".into())), raise(2));
        assert_eq!(
            eval(&left_raise, &mut vec![]).unwrap_err(),
            RuntimeError::UncaughtException("1".into())
        );
        assert_eq!(
            eval(&right_raise, &mut vec![]).unwrap_err(),
            RuntimeError::UncaughtException("2".into())
        );
    }

    #[test]
    fn nested_raise_propagates_the_inner_exception() {
        let inner = Term::Raise {
            value: Box::new(Term::Exception(Box::new(Term::Nat(7)))),
            result_type: Type::Exn,
        };
        let outer = Term::Raise {
            value: Box::new(inner),
            result_type: Type::Nat,
        };
        assert_eq!(
            eval(&outer, &mut vec![]).unwrap_err(),
            RuntimeError::UncaughtException("7".into())
        );
    }

    #[test]
    fn successful_try_does_not_evaluate_its_handler() {
        let ordinary = Term::Try {
            body: Box::new(Term::Nat(5)),
            handler: Box::new(Term::Error),
        };
        let carrying = Term::TryWith {
            body: Box::new(Term::Nat(6)),
            handler: Box::new(Term::Error),
        };
        assert_eq!(eval(&ordinary, &mut vec![]).unwrap().as_nat(), Some(5));
        assert_eq!(eval(&carrying, &mut vec![]).unwrap().as_nat(), Some(6));
    }

    #[test]
    fn divergent_fix_hits_the_configured_limit() {
        let identity = abs("x", Type::Nat, Term::Var("x".into()));
        let term = Term::Fix(Box::new(identity));
        assert_eq!(
            eval_with_limit(&term, &mut vec![], 30).unwrap_err(),
            RuntimeError::StepLimit
        );
    }
}
