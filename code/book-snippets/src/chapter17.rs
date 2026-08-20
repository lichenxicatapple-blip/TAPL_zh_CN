//! Rust counterparts for the OCaml fragments in Chapter 17.

// TAPL-SNIPPET-BEGIN: ch17-syntax
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Type {
    Top,
    Arrow(Box<Type>, Box<Type>),
    Record(Vec<(String, Type)>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    Record(Vec<(String, Term)>),
    Projection(Box<Term>, String),
    Variable(usize),
    Abstraction(String, Type, Box<Term>),
    Application(Box<Term>, Box<Term>),
}

impl std::fmt::Display for Type {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Type::Top => write!(formatter, "Top"),
            Type::Arrow(input, output) => write!(formatter, "({input} -> {output})"),
            Type::Record(fields) => {
                write!(formatter, "{{")?;
                for (index, (label, ty)) in fields.iter().enumerate() {
                    if index > 0 {
                        write!(formatter, ", ")?;
                    }
                    write!(formatter, "{label}: {ty}")?;
                }
                write!(formatter, "}}")
            }
        }
    }
}
// TAPL-SNIPPET-END: ch17-syntax

// TAPL-SNIPPET-BEGIN: ch17-support
pub type Context = Vec<Type>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeError {
    UnboundVariable(usize),
    MissingField(String),
    ExpectedRecord(Type),
    ExpectedFunction(Type),
    ParameterMismatch { expected: Type, actual: Type },
}
// TAPL-SNIPPET-END: ch17-support

// TAPL-SNIPPET-BEGIN: ch17-subtype
pub fn is_subtype(source: &Type, target: &Type) -> bool {
    if source == target {
        return true;
    }

    match (source, target) {
        (_, Type::Top) => true,
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
        Term::Record(fields) => fields
            .iter()
            .map(|(label, field)| Ok((label.clone(), type_of(context, field)?)))
            .collect::<Result<Vec<_>, _>>()
            .map(Type::Record),
        Term::Projection(record, label) => match type_of(context, record)? {
            Type::Record(fields) => fields
                .iter()
                .find(|(field_label, _)| field_label == label)
                .map(|(_, field_type)| field_type.clone())
                .ok_or_else(|| TypeError::MissingField(label.clone())),
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
        Term::Application(function, argument) => {
            let function_type = type_of(context, function)?;
            let argument_type = type_of(context, argument)?;
            match function_type {
                Type::Arrow(parameter_type, result_type) => {
                    if is_subtype(&argument_type, &parameter_type) {
                        Ok(*result_type)
                    } else {
                        Err(TypeError::ParameterMismatch {
                            expected: *parameter_type,
                            actual: argument_type,
                        })
                    }
                }
                other => Err(TypeError::ExpectedFunction(other)),
            }
        }
    }
}
// TAPL-SNIPPET-END: ch17-type-of

// The following language is the explicit Boolean/conditional extension used
// only by the author's solution to Exercise 17.3.1.
// TAPL-SNIPPET-BEGIN: sol-author-17-join-support
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JoinType {
    Bool,
    Top,
    Arrow(Box<JoinType>, Box<JoinType>),
    Record(Vec<(String, JoinType)>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JoinTerm {
    True,
    False,
    If(Box<JoinTerm>, Box<JoinTerm>, Box<JoinTerm>),
    Typed(JoinType),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JoinTypeError {
    ExpectedBoolean(JoinType),
}
// TAPL-SNIPPET-END: sol-author-17-join-support

// TAPL-SNIPPET-BEGIN: sol-author-17-join
/// 计算两个类型的最小公共超类型（并）。
/// 结果同时是两个输入类型的超类型，并且在所有公共超类型中尽可能小；
/// 条件式用它合并两个分支的类型。
pub fn join(left: &JoinType, right: &JoinType) -> JoinType {
    match (left, right) {
        (JoinType::Bool, JoinType::Bool) => JoinType::Bool,
        (JoinType::Arrow(left_in, left_out), JoinType::Arrow(right_in, right_out)) => {
            meet(left_in, right_in).map_or(JoinType::Top, |input| {
                JoinType::Arrow(Box::new(input), Box::new(join(left_out, right_out)))
            })
        }
        (JoinType::Record(left_fields), JoinType::Record(right_fields)) => JoinType::Record(
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
        _ => JoinType::Top,
    }
}
// TAPL-SNIPPET-END: sol-author-17-join

// TAPL-SNIPPET-BEGIN: sol-author-17-meet
/// 计算两个类型的最大公共子类型（交）。
/// 结果同时是两个输入类型的子类型，并且在所有公共子类型中尽可能大；
/// 当前类型语言无法表示这样的类型时返回 `None`。
pub fn meet(left: &JoinType, right: &JoinType) -> Option<JoinType> {
    match (left, right) {
        (JoinType::Top, other) | (other, JoinType::Top) => Some(other.clone()),
        (JoinType::Bool, JoinType::Bool) => Some(JoinType::Bool),
        (JoinType::Arrow(left_in, left_out), JoinType::Arrow(right_in, right_out)) => {
            Some(JoinType::Arrow(
                Box::new(join(left_in, right_in)),
                Box::new(meet(left_out, right_out)?),
            ))
        }
        (JoinType::Record(left_fields), JoinType::Record(right_fields)) => {
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
            Some(JoinType::Record(result))
        }
        _ => None,
    }
}
// TAPL-SNIPPET-END: sol-author-17-meet

// TAPL-SNIPPET-BEGIN: sol-author-17-conditional-type
/// 计算布尔值与条件式扩展中的项类型。
/// 条件式的守卫必须具有 `Bool` 类型，整个条件式的类型是两个分支类型的并。
pub fn extended_type_of(term: &JoinTerm) -> Result<JoinType, JoinTypeError> {
    match term {
        JoinTerm::True | JoinTerm::False => Ok(JoinType::Bool),
        JoinTerm::If(guard, then_term, else_term) => {
            let guard_type = extended_type_of(guard)?;
            if guard_type != JoinType::Bool {
                return Err(JoinTypeError::ExpectedBoolean(guard_type));
            }
            Ok(join(
                &extended_type_of(then_term)?,
                &extended_type_of(else_term)?,
            ))
        }
        JoinTerm::Typed(ty) => Ok(ty.clone()),
    }
}
// TAPL-SNIPPET-END: sol-author-17-conditional-type

// TAPL-SNIPPET-BEGIN: sol-translator-17-diagnostic-error
/// 描述子类型检查和类型检查失败的准确位置与原因。
///
/// 与只返回 `false` 的子类型判断不同，这些变体会保存沿嵌套记录或函数类型
/// 走过的路径，以及出错位置的实际类型和预期类型。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticError {
    /// 源记录缺少目标记录要求的字段。
    MissingSubtypeField {
        path: Vec<String>,
        label: String,
        source: Type,
        target: Type,
    },
    /// 对应位置的类型构造不同，例如一侧是记录而另一侧是函数。
    ShapeMismatch {
        path: Vec<String>,
        source: Type,
        target: Type,
    },
    /// de Bruijn 索引超出了当前上下文。
    UnboundVariable { index: usize, context_len: usize },
    /// 投影所选字段不存在。
    MissingProjectionField { label: String, record: Type },
    /// 投影的左侧不是记录。
    ExpectedRecord(Type),
    /// 应用的左侧不是函数。
    ExpectedFunction(Type),
    /// 应用实参不满足函数参数类型；内部错误保留具体的子类型失败原因。
    ParameterMismatch(Box<DiagnosticError>),
}
// TAPL-SNIPPET-END: sol-translator-17-diagnostic-error

/// 在嵌套类型路径后追加一个分量，并保持调用者传入的路径不变。
fn extend_path(path: &[String], part: impl Into<String>) -> Vec<String> {
    let mut extended = path.to_vec();
    extended.push(part.into());
    extended
}

/// 把诊断路径格式化为面向读者的字段路径。
fn diagnostic_path(path: &[String]) -> String {
    if path.is_empty() {
        "<root>".to_owned()
    } else {
        path.join(".")
    }
}

impl std::fmt::Display for DiagnosticError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticError::MissingSubtypeField {
                path,
                label,
                source,
                target,
            } => write!(
                formatter,
                "below {}: actual type {source} lacks field {label} required by {target}",
                diagnostic_path(path)
            ),
            DiagnosticError::ShapeMismatch {
                path,
                source,
                target,
            } => write!(
                formatter,
                "below {}: actual type {source} is incompatible with expected type {target}",
                diagnostic_path(path)
            ),
            DiagnosticError::UnboundVariable { index, context_len } => write!(
                formatter,
                "variable {index} is outside a context of length {context_len}"
            ),
            DiagnosticError::MissingProjectionField { label, record } => {
                write!(formatter, "record type {record} has no field {label}")
            }
            DiagnosticError::ExpectedRecord(actual) => {
                write!(
                    formatter,
                    "projection expected a record, but found {actual}"
                )
            }
            DiagnosticError::ExpectedFunction(actual) => {
                write!(
                    formatter,
                    "application expected a function, but found {actual}"
                )
            }
            DiagnosticError::ParameterMismatch(error) => {
                write!(formatter, "argument type mismatch: {error}")
            }
        }
    }
}

// TAPL-SNIPPET-BEGIN: sol-translator-17-diagnostic-subtype
/// 检查 `source <: target`，并在首次失败的位置返回结构化诊断。
///
/// `path` 记录递归检查已经进入的字段或函数位置；调用入口通常传入空切片。
/// 函数参数沿逆变方向递归，函数结果和记录字段沿协变方向递归。
pub fn check_subtype(path: &[String], source: &Type, target: &Type) -> Result<(), DiagnosticError> {
    if source == target {
        return Ok(());
    }
    match (source, target) {
        (_, Type::Top) => Ok(()),
        (Type::Arrow(source_in, source_out), Type::Arrow(target_in, target_out)) => {
            check_subtype(&extend_path(path, "parameter"), target_in, source_in)?;
            check_subtype(&extend_path(path, "result"), source_out, target_out)
        }
        (Type::Record(source_fields), Type::Record(target_fields)) => {
            for (label, target_type) in target_fields {
                let (_, source_type) = source_fields
                    .iter()
                    .find(|(source_label, _)| source_label == label)
                    .ok_or_else(|| DiagnosticError::MissingSubtypeField {
                        path: path.to_vec(),
                        label: label.clone(),
                        source: source.clone(),
                        target: target.clone(),
                    })?;
                check_subtype(&extend_path(path, label.clone()), source_type, target_type)?;
            }
            Ok(())
        }
        _ => Err(DiagnosticError::ShapeMismatch {
            path: path.to_vec(),
            source: source.clone(),
            target: target.clone(),
        }),
    }
}
// TAPL-SNIPPET-END: sol-translator-17-diagnostic-subtype

// TAPL-SNIPPET-BEGIN: sol-translator-17-diagnostic-app
/// 检查一个应用项并返回其结果类型。
///
/// 这里先分别计算函数与实参的类型，再用带路径的子类型检查验证实参；失败时
/// `ParameterMismatch` 会保留内部的字段路径或类型构造差异。
fn diagnostic_application_type(
    context: &Context,
    function: &Term,
    argument: &Term,
) -> Result<Type, DiagnosticError> {
    let function_type = diagnostic_type_of(context, function)?;
    let argument_type = diagnostic_type_of(context, argument)?;
    match function_type {
        Type::Arrow(parameter_type, result_type) => {
            check_subtype(&[], &argument_type, &parameter_type)
                .map_err(|error| DiagnosticError::ParameterMismatch(Box::new(error)))?;
            Ok(*result_type)
        }
        other => Err(DiagnosticError::ExpectedFunction(other)),
    }
}
// TAPL-SNIPPET-END: sol-translator-17-diagnostic-app

/// 计算项的类型，并把变量、投影、应用和子类型失败转换为精确诊断。
///
/// 返回值与本章正文的 `type_of` 相同；区别只在于错误分支保留了足以定位问题的
/// 类型和路径信息。
pub fn diagnostic_type_of(context: &Context, term: &Term) -> Result<Type, DiagnosticError> {
    match term {
        Term::Record(fields) => fields
            .iter()
            .map(|(label, field)| Ok((label.clone(), diagnostic_type_of(context, field)?)))
            .collect::<Result<Vec<_>, _>>()
            .map(Type::Record),
        Term::Projection(record, label) => match diagnostic_type_of(context, record)? {
            Type::Record(fields) => fields
                .iter()
                .find(|(field_label, _)| field_label == label)
                .map(|(_, field_type)| field_type.clone())
                .ok_or_else(|| DiagnosticError::MissingProjectionField {
                    label: label.clone(),
                    record: Type::Record(fields.clone()),
                }),
            other => Err(DiagnosticError::ExpectedRecord(other)),
        },
        Term::Variable(index) => {
            context
                .get(*index)
                .cloned()
                .ok_or(DiagnosticError::UnboundVariable {
                    index: *index,
                    context_len: context.len(),
                })
        }
        Term::Abstraction(_, parameter_type, body) => {
            let mut body_context = context.clone();
            body_context.insert(0, parameter_type.clone());
            Ok(Type::Arrow(
                Box::new(parameter_type.clone()),
                Box::new(diagnostic_type_of(&body_context, body)?),
            ))
        }
        Term::Application(function, argument) => {
            diagnostic_application_type(context, function, argument)
        }
    }
}

// TAPL-SNIPPET-BEGIN: sol-translator-17-coercion-support
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TargetType {
    Unit,
    Arrow(Box<TargetType>, Box<TargetType>),
    Record(Vec<(String, TargetType)>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TargetTerm {
    Unit,
    Variable(usize),
    Abstraction(TargetType, Box<TargetTerm>),
    Application(Box<TargetTerm>, Box<TargetTerm>),
    Record(Vec<(String, TargetTerm)>),
    Projection(Box<TargetTerm>, String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TranslationError {
    NotSubtype { source: Type, target: Type },
    UnboundVariable(usize),
    MissingField(String),
    ExpectedRecord(Type),
    ExpectedFunction(Type),
}

/// 把源语言类型递归映射为不含子类型关系的目标语言类型。
///
/// 翻译结果用于目标项的类型标注、目标语言上下文，以及 `coerce` 所生成函数的
/// 输入和输出类型。这个函数只改变类型的表示，不构造实际的值转换；`Top` 被
/// 擦除为只有一个值的 `Unit`，箭头与记录则递归翻译其组成部分。实际的值转换
/// 由 `coerce` 生成。
pub fn translate_type(source: &Type) -> TargetType {
    match source {
        Type::Top => TargetType::Unit,
        Type::Arrow(input, output) => TargetType::Arrow(
            Box::new(translate_type(input)),
            Box::new(translate_type(output)),
        ),
        Type::Record(fields) => TargetType::Record(
            fields
                .iter()
                .map(|(label, ty)| (label.clone(), translate_type(ty)))
                .collect(),
        ),
    }
}
// TAPL-SNIPPET-END: sol-translator-17-coercion-support

// TAPL-SNIPPET-BEGIN: sol-translator-17-coercion
/// 根据 `source <: target` 构造一个显式强制转换函数。
///
/// 返回的目标项具有 `translate_type(source) -> translate_type(target)` 类型。
/// 若源类型并非目标类型的子类型，则返回 `TranslationError::NotSubtype`。
pub fn coerce(source: &Type, target: &Type) -> Result<TargetTerm, TranslationError> {
    let source_target_type = translate_type(source);
    // 相同类型之间的强制转换就是恒等函数。
    if source == target {
        return Ok(TargetTerm::Abstraction(
            source_target_type,
            Box::new(TargetTerm::Variable(0)),
        ));
    }
    match (source, target) {
        // `Top` 翻译成 `Unit`，因此转换只需丢弃输入并返回 `unit`。
        (_, Type::Top) => Ok(TargetTerm::Abstraction(
            source_target_type,
            Box::new(TargetTerm::Unit),
        )),
        (Type::Arrow(source_in, source_out), Type::Arrow(target_in, target_out)) => {
            // 要把原函数 f : source_in -> source_out 包装成
            // target_in -> target_out，箭头子类型规则要求：
            //
            //   target_in <: source_in
            //   source_out <: target_out
            //
            // 因此生成的包装函数具有以下结构：
            //
            //   lambda f. lambda x.
            //     output_coercion (f (input_coercion x))
            //
            // 新函数收到 target_in 型参数 x，先把它转换成原函数能够接受的
            // source_in；原函数返回 source_out 后，再转换成承诺的 target_out。
            let input_coercion = coerce(target_in, source_in)?;
            let output_coercion = coerce(source_out, target_out)?;

            // 两层抽象依次绑定 f 和 x。在最内层函数体中，Variable(0) 是 x，
            // Variable(1) 是外层的 f。
            Ok(TargetTerm::Abstraction(
                translate_type(source),
                Box::new(TargetTerm::Abstraction(
                    translate_type(target_in),
                    Box::new(TargetTerm::Application(
                        Box::new(output_coercion),
                        Box::new(TargetTerm::Application(
                            Box::new(TargetTerm::Variable(1)),
                            Box::new(TargetTerm::Application(
                                Box::new(input_coercion),
                                Box::new(TargetTerm::Variable(0)),
                            )),
                        )),
                    )),
                )),
            ))
        }
        (Type::Record(source_fields), Type::Record(target_fields)) => {
            // 目标语言没有记录子类型规则，因此不能直接把带有额外字段的源记录
            // 当作目标记录使用。这里生成如下形式的包装函数：
            //
            //   lambda r. {
            //     l1 = c1 (r.l1),
            //     ...
            //   }
            //
            // 输出必须恰好提供目标类型要求的字段，所以遍历 target_fields；
            // 源记录的额外字段被丢弃，这对应记录宽度子类型化。
            let translated_fields = target_fields
                .iter()
                .map(|(label, target_type)| {
                    // 每个目标字段都必须能在源记录中找到同名字段。
                    let (_, source_type) = source_fields
                        .iter()
                        .find(|(source_label, _)| source_label == label)
                        .ok_or_else(|| TranslationError::MissingField(label.clone()))?;

                    // 保留下来的字段也可能需要从 source_type 转成 target_type；
                    // 这对应记录深度子类型化。
                    let field_coercion = coerce(source_type, target_type)?;
                    Ok((
                        label.clone(),
                        // 当前只有一层 lambda，Variable(0) 就是源记录 r。
                        // 这一项表示 c_i (r.l_i)。
                        TargetTerm::Application(
                            Box::new(field_coercion),
                            Box::new(TargetTerm::Projection(
                                Box::new(TargetTerm::Variable(0)),
                                label.clone(),
                            )),
                        ),
                    ))
                })
                .collect::<Result<Vec<_>, TranslationError>>()?;
            Ok(TargetTerm::Abstraction(
                translate_type(source),
                Box::new(TargetTerm::Record(translated_fields)),
            ))
        }
        _ => Err(TranslationError::NotSubtype {
            source: source.clone(),
            target: target.clone(),
        }),
    }
}
// TAPL-SNIPPET-END: sol-translator-17-coercion

// TAPL-SNIPPET-BEGIN: sol-translator-17-translate
/// 翻译函数应用，并在实参外显式插入所需的强制转换。
///
/// 返回值同时给出目标语言中的结果类型和应用项。其余项构造由
/// `translate_term` 逐层递归翻译。
fn translate_application(
    context: &Context,
    function: &Term,
    argument: &Term,
) -> Result<(TargetType, TargetTerm), TranslationError> {
    let source_function_type =
        type_of(context, function).map_err(|_| TranslationError::ExpectedFunction(Type::Top))?;
    let source_argument_type =
        type_of(context, argument).map_err(|_| TranslationError::ExpectedFunction(Type::Top))?;
    let (_, function_term) = translate_term(context, function)?;
    let (_, argument_term) = translate_term(context, argument)?;
    match source_function_type {
        Type::Arrow(parameter_type, result_type) => {
            // 把实参从其实际类型显式转换为函数要求的参数类型。
            let argument_coercion = coerce(&source_argument_type, &parameter_type)?;
            Ok((
                translate_type(&result_type),
                TargetTerm::Application(
                    Box::new(function_term),
                    Box::new(TargetTerm::Application(
                        Box::new(argument_coercion),
                        Box::new(argument_term),
                    )),
                ),
            ))
        }
        other => Err(TranslationError::ExpectedFunction(other)),
    }
}
// TAPL-SNIPPET-END: sol-translator-17-translate

/// 翻译一个源语言项，同时返回其目标类型与插入强制转换后的目标项。
///
/// 结构相同的语法形式递归翻译；函数应用是关键情形，其中实参会先被转换为
/// 函数参数所要求的类型。
pub fn translate_term(
    context: &Context,
    term: &Term,
) -> Result<(TargetType, TargetTerm), TranslationError> {
    match term {
        Term::Record(fields) => {
            let translated = fields
                .iter()
                .map(|(label, field)| {
                    let (field_type, field_term) = translate_term(context, field)?;
                    Ok((label.clone(), field_type, field_term))
                })
                .collect::<Result<Vec<_>, TranslationError>>()?;
            Ok((
                TargetType::Record(
                    translated
                        .iter()
                        .map(|(label, ty, _)| (label.clone(), ty.clone()))
                        .collect(),
                ),
                TargetTerm::Record(
                    translated
                        .into_iter()
                        .map(|(label, _, field)| (label, field))
                        .collect(),
                ),
            ))
        }
        Term::Projection(record, label) => {
            let (record_type, record_term) = translate_term(context, record)?;
            match record_type {
                TargetType::Record(fields) => {
                    let (_, field_type) = fields
                        .iter()
                        .find(|(field_label, _)| field_label == label)
                        .ok_or_else(|| TranslationError::MissingField(label.clone()))?;
                    Ok((
                        field_type.clone(),
                        TargetTerm::Projection(Box::new(record_term), label.clone()),
                    ))
                }
                _ => Err(TranslationError::ExpectedRecord(
                    type_of(context, record)
                        .map_err(|_| TranslationError::ExpectedRecord(Type::Top))?,
                )),
            }
        }
        Term::Variable(index) => {
            let ty = context
                .get(*index)
                .ok_or(TranslationError::UnboundVariable(*index))?;
            Ok((translate_type(ty), TargetTerm::Variable(*index)))
        }
        Term::Abstraction(_, parameter_type, body) => {
            let mut body_context = context.clone();
            body_context.insert(0, parameter_type.clone());
            let (body_type, body_term) = translate_term(&body_context, body)?;
            let parameter_target_type = translate_type(parameter_type);
            Ok((
                TargetType::Arrow(Box::new(parameter_target_type.clone()), Box::new(body_type)),
                TargetTerm::Abstraction(parameter_target_type, Box::new(body_term)),
            ))
        }
        Term::Application(function, argument) => translate_application(context, function, argument),
    }
}

/// 在目标项中移动自由变量索引；`cutoff` 以下的索引受当前绑定子保护。
fn target_shift_walk(distance: isize, cutoff: usize, term: &TargetTerm) -> TargetTerm {
    match term {
        TargetTerm::Unit => TargetTerm::Unit,
        TargetTerm::Variable(index) => {
            if *index >= cutoff {
                TargetTerm::Variable(index.checked_add_signed(distance).expect("valid shift"))
            } else {
                TargetTerm::Variable(*index)
            }
        }
        TargetTerm::Abstraction(ty, body) => TargetTerm::Abstraction(
            ty.clone(),
            Box::new(target_shift_walk(distance, cutoff + 1, body)),
        ),
        TargetTerm::Application(function, argument) => TargetTerm::Application(
            Box::new(target_shift_walk(distance, cutoff, function)),
            Box::new(target_shift_walk(distance, cutoff, argument)),
        ),
        TargetTerm::Record(fields) => TargetTerm::Record(
            fields
                .iter()
                .map(|(label, field)| (label.clone(), target_shift_walk(distance, cutoff, field)))
                .collect(),
        ),
        TargetTerm::Projection(record, label) => TargetTerm::Projection(
            Box::new(target_shift_walk(distance, cutoff, record)),
            label.clone(),
        ),
    }
}

/// 从最外层开始移动目标项中的自由变量索引。
fn target_shift(distance: isize, term: &TargetTerm) -> TargetTerm {
    target_shift_walk(distance, 0, term)
}

/// 在目标项中执行避免捕获的 de Bruijn 替换。
fn target_substitute_walk(
    variable: usize,
    replacement: &TargetTerm,
    cutoff: usize,
    term: &TargetTerm,
) -> TargetTerm {
    match term {
        TargetTerm::Unit => TargetTerm::Unit,
        TargetTerm::Variable(index) if *index == variable + cutoff => {
            target_shift(isize::try_from(cutoff).expect("cutoff fits"), replacement)
        }
        TargetTerm::Variable(index) => TargetTerm::Variable(*index),
        TargetTerm::Abstraction(ty, body) => TargetTerm::Abstraction(
            ty.clone(),
            Box::new(target_substitute_walk(
                variable,
                replacement,
                cutoff + 1,
                body,
            )),
        ),
        TargetTerm::Application(function, argument) => TargetTerm::Application(
            Box::new(target_substitute_walk(
                variable,
                replacement,
                cutoff,
                function,
            )),
            Box::new(target_substitute_walk(
                variable,
                replacement,
                cutoff,
                argument,
            )),
        ),
        TargetTerm::Record(fields) => TargetTerm::Record(
            fields
                .iter()
                .map(|(label, field)| {
                    (
                        label.clone(),
                        target_substitute_walk(variable, replacement, cutoff, field),
                    )
                })
                .collect(),
        ),
        TargetTerm::Projection(record, label) => TargetTerm::Projection(
            Box::new(target_substitute_walk(
                variable,
                replacement,
                cutoff,
                record,
            )),
            label.clone(),
        ),
    }
}

/// 用给定项替换目标函数体最外层绑定的变量。
fn target_substitute_top(replacement: &TargetTerm, body: &TargetTerm) -> TargetTerm {
    let lifted = target_shift(1, replacement);
    let substituted = target_substitute_walk(0, &lifted, 0, body);
    target_shift(-1, &substituted)
}

/// 判断目标项是否已是按值求值策略下的值。
fn target_is_value(term: &TargetTerm) -> bool {
    match term {
        TargetTerm::Unit | TargetTerm::Abstraction(_, _) => true,
        TargetTerm::Record(fields) => fields.iter().all(|(_, field)| target_is_value(field)),
        _ => false,
    }
}

/// 对翻译后的目标项执行一步按值求值。
pub fn target_eval1(term: &TargetTerm) -> Option<TargetTerm> {
    match term {
        TargetTerm::Application(function, argument) => {
            if let TargetTerm::Abstraction(_, body) = function.as_ref()
                && target_is_value(argument)
            {
                return Some(target_substitute_top(argument, body));
            }
            if !target_is_value(function) {
                return target_eval1(function)
                    .map(|next| TargetTerm::Application(Box::new(next), argument.clone()));
            }
            target_eval1(argument)
                .map(|next| TargetTerm::Application(function.clone(), Box::new(next)))
        }
        TargetTerm::Record(fields) => {
            for (index, (_, field)) in fields.iter().enumerate() {
                if !target_is_value(field) {
                    let next = target_eval1(field)?;
                    let mut result = fields.clone();
                    result[index].1 = next;
                    return Some(TargetTerm::Record(result));
                }
            }
            None
        }
        TargetTerm::Projection(record, label) => {
            if !target_is_value(record) {
                return target_eval1(record)
                    .map(|next| TargetTerm::Projection(Box::new(next), label.clone()));
            }
            if let TargetTerm::Record(fields) = record.as_ref() {
                return fields
                    .iter()
                    .find(|(field_label, _)| field_label == label)
                    .map(|(_, field)| field.clone());
            }
            None
        }
        _ => None,
    }
}

/// 反复执行单步求值，直到目标项成为值或无法继续求值。
pub fn target_eval(term: &TargetTerm) -> TargetTerm {
    let mut current = term.clone();
    while let Some(next) = target_eval1(&current) {
        current = next;
    }
    current
}

/// 检查目标项的类型；测试用它验证翻译确实消除了子类型规则。
fn target_type_of(context: &[TargetType], term: &TargetTerm) -> Option<TargetType> {
    match term {
        TargetTerm::Unit => Some(TargetType::Unit),
        TargetTerm::Variable(index) => context.get(*index).cloned(),
        TargetTerm::Abstraction(parameter_type, body) => {
            let mut body_context = context.to_vec();
            body_context.insert(0, parameter_type.clone());
            Some(TargetType::Arrow(
                Box::new(parameter_type.clone()),
                Box::new(target_type_of(&body_context, body)?),
            ))
        }
        TargetTerm::Application(function, argument) => match target_type_of(context, function)? {
            TargetType::Arrow(parameter_type, result_type)
                if *parameter_type == target_type_of(context, argument)? =>
            {
                Some(*result_type)
            }
            _ => None,
        },
        TargetTerm::Record(fields) => Some(TargetType::Record(
            fields
                .iter()
                .map(|(label, field)| Some((label.clone(), target_type_of(context, field)?)))
                .collect::<Option<Vec<_>>>()?,
        )),
        TargetTerm::Projection(record, label) => match target_type_of(context, record)? {
            TargetType::Record(fields) => fields
                .into_iter()
                .find(|(field_label, _)| field_label == label)
                .map(|(_, field_type)| field_type),
            _ => None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_record() -> Type {
        Type::Record(Vec::new())
    }

    fn x_top_record() -> Type {
        Type::Record(vec![("x".into(), Type::Top)])
    }

    #[test]
    fn record_width_and_nested_depth_are_checked() {
        let source = Type::Record(vec![
            (
                "x".into(),
                Type::Record(vec![("a".into(), Type::Top), ("b".into(), Type::Top)]),
            ),
            ("y".into(), Type::Top),
        ]);
        let target = Type::Record(vec![(
            "x".into(),
            Type::Record(vec![("a".into(), Type::Top)]),
        )]);
        assert!(is_subtype(&source, &target));
        assert!(!is_subtype(&target, &source));
    }

    #[test]
    fn missing_fields_and_arrow_mismatches_are_rejected() {
        assert!(!is_subtype(&empty_record(), &x_top_record()));
        let record_function = Type::Arrow(Box::new(x_top_record()), Box::new(Type::Top));
        let top_function = Type::Arrow(Box::new(Type::Top), Box::new(Type::Top));
        assert!(is_subtype(&top_function, &record_function));
        assert!(!is_subtype(&record_function, &top_function));
    }

    #[test]
    fn application_accepts_a_subtype_argument() {
        let function = Term::Abstraction(
            "r".into(),
            x_top_record(),
            Box::new(Term::Projection(Box::new(Term::Variable(0)), "x".into())),
        );
        let argument = Term::Record(vec![
            ("x".into(), Term::Record(Vec::new())),
            ("y".into(), Term::Record(Vec::new())),
        ]);
        assert_eq!(
            type_of(
                &Vec::new(),
                &Term::Application(Box::new(function), Box::new(argument)),
            ),
            Ok(Type::Top)
        );
    }

    #[test]
    fn joins_and_meets_cover_success_and_failure() {
        let left = JoinType::Record(vec![
            ("x".into(), JoinType::Bool),
            ("y".into(), JoinType::Top),
        ]);
        let right = JoinType::Record(vec![
            ("x".into(), JoinType::Bool),
            ("z".into(), JoinType::Top),
        ]);
        assert_eq!(
            join(&left, &right),
            JoinType::Record(vec![("x".into(), JoinType::Bool)])
        );
        assert_eq!(meet(&JoinType::Top, &JoinType::Bool), Some(JoinType::Bool));
        assert_eq!(meet(&JoinType::Bool, &JoinType::Record(Vec::new())), None);
        assert_eq!(
            join(
                &JoinType::Arrow(Box::new(JoinType::Bool), Box::new(JoinType::Top)),
                &JoinType::Arrow(
                    Box::new(JoinType::Record(Vec::new())),
                    Box::new(JoinType::Top),
                ),
            ),
            JoinType::Top
        );
    }

    #[test]
    fn conditionals_check_the_guard_and_join_branches() {
        let conditional = JoinTerm::If(
            Box::new(JoinTerm::True),
            Box::new(JoinTerm::Typed(JoinType::Bool)),
            Box::new(JoinTerm::Typed(JoinType::Top)),
        );
        assert_eq!(extended_type_of(&conditional), Ok(JoinType::Top));
        assert_eq!(
            extended_type_of(&JoinTerm::If(
                Box::new(JoinTerm::Typed(JoinType::Top)),
                Box::new(JoinTerm::True),
                Box::new(JoinTerm::False),
            )),
            Err(JoinTypeError::ExpectedBoolean(JoinType::Top))
        );
    }

    #[test]
    fn diagnostics_identify_nested_missing_fields_and_bad_variables() {
        let source = Type::Record(vec![("outer".into(), empty_record())]);
        let target = Type::Record(vec![("outer".into(), x_top_record())]);
        assert_eq!(
            check_subtype(&[], &source, &target),
            Err(DiagnosticError::MissingSubtypeField {
                path: vec!["outer".into()],
                label: "x".into(),
                source: empty_record(),
                target: x_top_record(),
            })
        );
        assert_eq!(
            diagnostic_type_of(&Vec::new(), &Term::Variable(0)),
            Err(DiagnosticError::UnboundVariable {
                index: 0,
                context_len: 0,
            })
        );
    }

    #[test]
    fn diagnostics_render_actual_expected_types_and_projection_failures() {
        let mismatch = check_subtype(
            &["payload".into()],
            &empty_record(),
            &Type::Arrow(Box::new(Type::Top), Box::new(Type::Top)),
        )
        .unwrap_err()
        .to_string();
        assert!(mismatch.contains("below payload"));
        assert!(mismatch.contains("actual type {}"));
        assert!(mismatch.contains("expected type (Top -> Top)"));

        let missing = diagnostic_type_of(
            &Vec::new(),
            &Term::Projection(Box::new(Term::Record(Vec::new())), "x".into()),
        )
        .unwrap_err();
        assert_eq!(missing.to_string(), "record type {} has no field x");

        let non_record = diagnostic_type_of(
            &Vec::new(),
            &Term::Projection(
                Box::new(Term::Abstraction(
                    "x".into(),
                    Type::Top,
                    Box::new(Term::Variable(0)),
                )),
                "x".into(),
            ),
        )
        .unwrap_err();
        assert!(
            non_record
                .to_string()
                .contains("projection expected a record, but found (Top -> Top)")
        );

        let non_function = diagnostic_type_of(
            &Vec::new(),
            &Term::Application(
                Box::new(Term::Record(Vec::new())),
                Box::new(Term::Record(Vec::new())),
            ),
        )
        .unwrap_err();
        assert_eq!(
            non_function.to_string(),
            "application expected a function, but found {}"
        );
    }

    // TAPL-SNIPPET-BEGIN: sol-translator-17-example
    #[test]
    fn coercion_translation_is_typed_and_evaluates() {
        let function = Term::Abstraction(
            "r".into(),
            x_top_record(),
            Box::new(Term::Projection(Box::new(Term::Variable(0)), "x".into())),
        );
        let argument = Term::Record(vec![
            ("x".into(), Term::Record(Vec::new())),
            ("y".into(), Term::Record(Vec::new())),
        ]);
        let source = Term::Application(Box::new(function), Box::new(argument));
        let (translated_type, translated_term) = translate_term(&Vec::new(), &source).unwrap();
        assert_eq!(translated_type, TargetType::Unit);
        assert_eq!(
            target_type_of(&[], &translated_term),
            Some(TargetType::Unit)
        );
        assert_eq!(target_eval(&translated_term), TargetTerm::Unit);
    }
    // TAPL-SNIPPET-END: sol-translator-17-example
}
