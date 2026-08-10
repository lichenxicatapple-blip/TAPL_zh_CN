//! Rust counterparts for the OCaml fragments in Chapter 17.

// TAPL-SNIPPET-BEGIN: ch17-syntax
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Bool,
    Top,
    Bottom,
    Arrow(Box<Type>, Box<Type>),
    Record(Vec<(String, Type)>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    True,
    False,
    If(Box<Term>, Box<Term>, Box<Term>),
    Record(Vec<(String, Term)>),
    Projection(Box<Term>, String),
    Variable(usize),
    Abstraction(String, Type, Box<Term>),
    Application(Box<Term>, Box<Term>),
}
// TAPL-SNIPPET-END: ch17-syntax

pub type Context = Vec<Type>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeError {
    UnboundVariable(usize),
    MissingField(String),
    ExpectedRecord(Type),
    ExpectedFunction(Type),
    ParameterMismatch { expected: Type, actual: Type },
    ExpectedBoolean(Type),
}

// TAPL-SNIPPET-BEGIN: ch17-subtype
pub fn is_subtype(source: &Type, target: &Type) -> bool {
    if source == target {
        return true;
    }

    match (source, target) {
        (_, Type::Top) | (Type::Bottom, _) => true,
        (Type::Arrow(source_in, source_out), Type::Arrow(target_in, target_out)) => {
            is_subtype(target_in, source_in) && is_subtype(source_out, target_out)
        }
        (Type::Record(source_fields), Type::Record(target_fields)) => {
            target_fields.iter().all(|(label, target_type)| {
                source_fields
                    .iter()
                    .find(|(source_label, _)| source_label == label)
                    .is_some_and(|(_, source_type)| is_subtype(source_type, target_type))
            })
        }
        _ => false,
    }
}
// TAPL-SNIPPET-END: ch17-subtype

// TAPL-SNIPPET-BEGIN: ch17-type-of
pub fn type_of(context: &Context, term: &Term) -> Result<Type, TypeError> {
    match term {
        Term::True | Term::False => Ok(Type::Bool),
        Term::Record(fields) => fields
            .iter()
            .map(|(label, field)| Ok((label.clone(), type_of(context, field)?)))
            .collect::<Result<Vec<_>, _>>()
            .map(Type::Record),
        Term::Projection(record, label) => match type_of(context, record)? {
            Type::Record(fields) => fields
                .into_iter()
                .find(|(field_label, _)| field_label == label)
                .map(|(_, field_type)| field_type)
                .ok_or_else(|| TypeError::MissingField(label.clone())),
            Type::Bottom => Ok(Type::Bottom),
            other => Err(TypeError::ExpectedRecord(other)),
        },
        Term::Variable(index) => context
            .get(*index)
            .cloned()
            .ok_or(TypeError::UnboundVariable(*index)),
        Term::Abstraction(_, parameter_type, body) => {
            let mut body_context = context.clone();
            body_context.insert(0, parameter_type.clone());
            Ok(Type::Arrow(
                Box::new(parameter_type.clone()),
                Box::new(type_of(&body_context, body)?),
            ))
        }
        Term::Application(function, argument) => match type_of(context, function)? {
            Type::Arrow(parameter_type, result_type) => {
                let argument_type = type_of(context, argument)?;
                if is_subtype(&argument_type, &parameter_type) {
                    Ok(*result_type)
                } else {
                    Err(TypeError::ParameterMismatch {
                        expected: *parameter_type,
                        actual: argument_type,
                    })
                }
            }
            Type::Bottom => Ok(Type::Bottom),
            other => Err(TypeError::ExpectedFunction(other)),
        },
        Term::If(guard, then_term, else_term) => {
            let guard_type = type_of(context, guard)?;
            if !is_subtype(&guard_type, &Type::Bool) {
                return Err(TypeError::ExpectedBoolean(guard_type));
            }
            Ok(join(
                &type_of(context, then_term)?,
                &type_of(context, else_term)?,
            ))
        }
    }
}
// TAPL-SNIPPET-END: ch17-type-of

// TAPL-SNIPPET-BEGIN: sol-author-17-join
pub fn join(left: &Type, right: &Type) -> Type {
    match (left, right) {
        (Type::Bool, Type::Bool) => Type::Bool,
        (Type::Arrow(left_in, left_out), Type::Arrow(right_in, right_out)) => {
            meet(left_in, right_in).map_or(Type::Top, |input| {
                Type::Arrow(Box::new(input), Box::new(join(left_out, right_out)))
            })
        }
        (Type::Record(left_fields), Type::Record(right_fields)) => Type::Record(
            left_fields
                .iter()
                .filter_map(|(label, left_type)| {
                    right_fields
                        .iter()
                        .find(|(right_label, _)| right_label == label)
                        .map(|(_, right_type)| (label.clone(), join(left_type, right_type)))
                })
                .collect(),
        ),
        _ => Type::Top,
    }
}
// TAPL-SNIPPET-END: sol-author-17-join

// TAPL-SNIPPET-BEGIN: sol-author-17-meet
pub fn meet(left: &Type, right: &Type) -> Option<Type> {
    match (left, right) {
        (Type::Top, other) | (other, Type::Top) => Some(other.clone()),
        (Type::Bool, Type::Bool) => Some(Type::Bool),
        (Type::Arrow(left_in, left_out), Type::Arrow(right_in, right_out)) => Some(Type::Arrow(
            Box::new(join(left_in, right_in)),
            Box::new(meet(left_out, right_out)?),
        )),
        (Type::Record(left_fields), Type::Record(right_fields)) => {
            let mut result = left_fields.clone();
            for (label, right_type) in right_fields {
                if let Some((_, result_type)) = result
                    .iter_mut()
                    .find(|(result_label, _)| result_label == label)
                {
                    *result_type = meet(result_type, right_type)?;
                } else {
                    result.push((label.clone(), right_type.clone()));
                }
            }
            Some(Type::Record(result))
        }
        _ => None,
    }
}
// TAPL-SNIPPET-END: sol-author-17-meet

// TAPL-SNIPPET-BEGIN: sol-author-17-conditional-type
pub fn conditional_type(
    guard: &Type,
    then_type: &Type,
    else_type: &Type,
) -> Result<Type, TypeError> {
    if is_subtype(guard, &Type::Bool) {
        Ok(join(then_type, else_type))
    } else {
        Err(TypeError::ExpectedBoolean(guard.clone()))
    }
}
// TAPL-SNIPPET-END: sol-author-17-conditional-type

#[cfg(test)]
mod tests {
    use super::*;

    fn nat_record() -> Type {
        Type::Record(vec![("x".into(), Type::Bool)])
    }

    #[test]
    fn record_width_and_depth_are_checked() {
        let wide = Type::Record(vec![("x".into(), Type::Bool), ("y".into(), Type::Top)]);
        assert!(is_subtype(&wide, &nat_record()));
        assert!(!is_subtype(&nat_record(), &wide));
    }

    #[test]
    fn arrow_inputs_are_contravariant() {
        let narrow_input = Type::Arrow(Box::new(nat_record()), Box::new(Type::Bool));
        let wide_input = Type::Arrow(Box::new(Type::Top), Box::new(Type::Bool));
        assert!(is_subtype(&wide_input, &narrow_input));
        assert!(!is_subtype(&narrow_input, &wide_input));
    }

    #[test]
    fn application_accepts_a_subtype_argument() {
        let function = Term::Abstraction(
            "r".into(),
            nat_record(),
            Box::new(Term::Projection(Box::new(Term::Variable(0)), "x".into())),
        );
        let argument = Term::Record(vec![("x".into(), Term::True), ("y".into(), Term::False)]);
        assert_eq!(
            type_of(
                &Vec::new(),
                &Term::Application(Box::new(function), Box::new(argument)),
            ),
            Ok(Type::Bool)
        );
    }

    #[test]
    fn joins_keep_only_common_record_labels() {
        let left = Type::Record(vec![("x".into(), Type::Bool), ("y".into(), Type::Bool)]);
        let right = Type::Record(vec![("x".into(), Type::Bool), ("z".into(), Type::Bool)]);
        assert_eq!(join(&left, &right), nat_record());
    }
}
