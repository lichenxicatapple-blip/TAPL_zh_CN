//! Type reconstruction for the simply typed lambda calculus (Chapter 22).

// TAPL-SNIPPET-BEGIN: ch22-types-generator
use std::collections::{BTreeMap, BTreeSet};

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
}

impl FreshVariables {
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

    /// 预扫描整棵项，只登记显式类型标注中已经出现的类型变量名。
    ///
    /// 必须在推断开始前完成扫描，否则先处理的子项可能生成一个与后续标注
    /// 同名的 `?Xn`，把两个本应无关的类型变量错误地视为同一个。
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
    UnsupportedInReconbase(&'static str),
}

/// 收集类型中出现的所有类型变量。
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

/// 递归应用替换，并沿 `X -> Y -> Nat` 这样的替换链一直展开。
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

/// 复合两个替换：先应用 `older`，再应用 `newer`。
fn compose(newer: &Substitution, older: &Substitution) -> Substitution {
    let mut composed = older
        .iter()
        .map(|(name, ty)| (name.clone(), apply(newer, ty)))
        .collect::<Substitution>();
    composed.extend(newer.clone());
    composed
}

/// 把一个已经求解的变量等式应用于其余所有约束。
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
/// 计算有限类型等式集的最一般合一子。
///
/// 本章重构的是有限简单类型，因此出现检查会拒绝
/// `X = X -> Nat` 这样的循环等式。
pub fn unify(mut constraints: Constraints) -> Result<Substitution, TypeError> {
    let mut solution = Substitution::new();
    // 每个待处理约束都已经应用了此前求得的替换。
    while let Some((left, right)) = constraints.pop() {
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
/// 为项生成候选结果类型，以及使该项可赋型所需满足的类型等式。
pub fn generate_constraints(
    context: &Context,
    term: &Term,
    fresh: &mut FreshVariables,
) -> Result<(Type, Constraints), TypeError> {
    match term {
        Term::True | Term::False => Ok((Type::Bool, vec![])),
        Term::Zero => Ok((Type::Nat, vec![])),
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
            annotation: Some(parameter_type),
            body,
        } => {
            let parameter_type = parameter_type.clone();
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
        // Unit、引用、赋值和 let 等后续扩展由完整工程继续处理。
        extension => generate_extension_constraints(context, extension, fresh),
    }
}
// TAPL-SNIPPET-END: ch22-constraints

fn generate_extension_constraints(
    context: &Context,
    term: &Term,
    fresh: &mut FreshVariables,
) -> Result<(Type, Constraints), TypeError> {
    match term {
        Term::Unit => Ok((Type::Unit, vec![])),
        Term::Abstraction {
            parameter,
            annotation: None,
            body,
        } => {
            let parameter_type = fresh.fresh();
            let mut body_context = context.clone();
            body_context.insert(parameter.clone(), parameter_type.clone());
            let (body_type, constraints) = generate_constraints(&body_context, body, fresh)?;
            Ok((
                Type::Arrow(Box::new(parameter_type), Box::new(body_type)),
                constraints,
            ))
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
        Term::True
        | Term::False
        | Term::Zero
        | Term::Successor(_)
        | Term::Predecessor(_)
        | Term::IsZero(_)
        | Term::If(_, _, _)
        | Term::Variable(_)
        | Term::Abstraction {
            annotation: Some(_),
            ..
        }
        | Term::Application(_, _) => unreachable!("core term handled by generate_constraints"),
    }
}

// TAPL-SNIPPET-BEGIN: ch22-principal-type
/// 先生成约束再进行合一，从而重构项的主类型。
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

// TAPL-SNIPPET-BEGIN: ch22-incremental-reconbase
/// 把替换作用于单态类型上下文中的每个绑定。
fn apply_monomorphic_context(substitution: &Substitution, context: &Context) -> Context {
    context
        .iter()
        .map(|(name, ty)| (name.clone(), apply(substitution, ty)))
        .collect()
}

/// 为 `reconbase` 的项增量求解约束，并返回替换与当前主类型。
///
/// 后处理的子项总是在前面所得替换更新过的上下文中检查。本函数只处理
/// 第 22.5 节 `reconbase` 支持的语法；遇到无标注抽象、`let` 等本题尚未
/// 处理的形式时，会明确报告不支持。
#[allow(clippy::too_many_lines)]
fn infer_reconbase_incremental(
    context: &Context,
    term: &Term,
    fresh: &mut FreshVariables,
) -> Result<(Substitution, Type), TypeError> {
    match term {
        Term::True | Term::False => Ok((Substitution::new(), Type::Bool)),
        Term::Zero => Ok((Substitution::new(), Type::Nat)),
        Term::Variable(name) => context
            .get(name)
            .cloned()
            .map(|ty| (Substitution::new(), ty))
            .ok_or_else(|| TypeError::UnknownVariable(name.clone())),
        Term::Abstraction {
            parameter,
            annotation: Some(parameter_type),
            body,
        } => {
            let mut body_context = context.clone();
            body_context.insert(parameter.clone(), parameter_type.clone());
            let (substitution, body_type) =
                infer_reconbase_incremental(&body_context, body, fresh)?;
            Ok((
                substitution.clone(),
                Type::Arrow(
                    Box::new(apply(&substitution, parameter_type)),
                    Box::new(apply(&substitution, &body_type)),
                ),
            ))
        }
        Term::Application(function, argument) => {
            let (function_substitution, function_type) =
                infer_reconbase_incremental(context, function, fresh)?;
            let argument_context = apply_monomorphic_context(&function_substitution, context);
            let (argument_substitution, argument_type) =
                infer_reconbase_incremental(&argument_context, argument, fresh)?;

            let result_type = fresh.fresh();
            let application_substitution = unify(vec![(
                apply(&argument_substitution, &function_type),
                Type::Arrow(Box::new(argument_type), Box::new(result_type.clone())),
            )])?;
            let substitution = compose(
                &application_substitution,
                &compose(&argument_substitution, &function_substitution),
            );
            // result_type 是本分支刚创建的新鲜变量，前两个替换不可能约束它；
            // 只需应用当前合一所得的替换，就能得到整个应用的结果类型。
            Ok((substitution, apply(&application_substitution, &result_type)))
        }
        Term::Successor(argument) | Term::Predecessor(argument) => {
            let (substitution, argument_type) =
                infer_reconbase_incremental(context, argument, fresh)?;
            let numeric = unify(vec![(argument_type, Type::Nat)])?;
            Ok((compose(&numeric, &substitution), Type::Nat))
        }
        Term::IsZero(argument) => {
            let (substitution, argument_type) =
                infer_reconbase_incremental(context, argument, fresh)?;
            let numeric = unify(vec![(argument_type, Type::Nat)])?;
            Ok((compose(&numeric, &substitution), Type::Bool))
        }
        Term::If(guard, then_term, else_term) => {
            let (guard_substitution, guard_type) =
                infer_reconbase_incremental(context, guard, fresh)?;
            let guard_check = unify(vec![(guard_type, Type::Bool)])?;
            let before_then = compose(&guard_check, &guard_substitution);

            let then_context = apply_monomorphic_context(&before_then, context);
            let (then_substitution, then_type) =
                infer_reconbase_incremental(&then_context, then_term, fresh)?;
            let before_else = compose(&then_substitution, &before_then);

            let else_context = apply_monomorphic_context(&before_else, context);
            let (else_substitution, else_type) =
                infer_reconbase_incremental(&else_context, else_term, fresh)?;
            let branch_check = unify(vec![(
                apply(&else_substitution, &then_type),
                else_type.clone(),
            )])?;
            let substitution = compose(&branch_check, &compose(&else_substitution, &before_else));
            Ok((substitution, apply(&branch_check, &else_type)))
        }
        Term::Abstraction {
            annotation: None, ..
        } => Err(TypeError::UnsupportedInReconbase(
            "unannotated lambda abstraction",
        )),
        Term::Unit
        | Term::Reference(_)
        | Term::Dereference(_)
        | Term::Assignment(_, _)
        | Term::Sequence(_, _)
        | Term::Let { .. } => Err(TypeError::UnsupportedInReconbase(
            "construct introduced after Section 22.5",
        )),
    }
}

/// 增量重构 `reconbase` 项的主类型。
pub fn principal_type_incremental(context: &Context, term: &Term) -> Result<Type, TypeError> {
    let mut fresh = FreshVariables::default();
    // 生成新变量前，先登记上下文以及整棵项的标注中已经使用的变量名。
    for ty in context.values() {
        fresh.reserve_type(ty);
    }
    fresh.reserve_term_annotations(term);
    // 辅助函数已经把累计替换应用于返回类型；最外层不再需要该替换。
    let (_, ty) = infer_reconbase_incremental(context, term, &mut fresh)?;
    Ok(ty)
}
// TAPL-SNIPPET-END: ch22-incremental-reconbase

// TAPL-SNIPPET-BEGIN: ch22-algorithm-w-support
/// 一个类型方案由受全称量化的类型变量集合及其类型体组成。
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
    // 只替换类型方案中的自由变量。若方案是 forall X. X -> Y，而外部替换为
    // {X := Nat, Y := Bool}，就必须忽略 X := Nat，因为 X 已由 forall X 绑定；
    // 最终结果应是 forall X. X -> Bool。
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
    // 每次使用多态变量时，都以一组全新的类型变量替换量化变量。
    let substitution = scheme
        .quantified
        .iter()
        .map(|name| (name.clone(), fresh.fresh()))
        .collect();
    apply(&substitution, &scheme.ty)
}

fn generalize(context: &SchemeContext, ty: Type) -> TypeScheme {
    // 只量化类型中自由、但上下文中不自由的变量，避免捕获环境施加的约束。
    let context_variables = context_variables(context);
    let quantified = type_variables(&ty)
        .difference(&context_variables)
        .cloned()
        .collect();
    TypeScheme { quantified, ty }
}
// TAPL-SNIPPET-END: ch22-algorithm-w-support

// TAPL-SNIPPET-BEGIN: ch22-algorithm-w
/// 依照本节概述的 let 多态算法逐步推断主类型。
///
/// 每次递归调用同时返回子项的类型，以及检查该子项时求得的最一般替换；
/// 检查后续子项前，先把这个替换应用于上下文。
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
        // 查询多态变量时实例化其类型方案；不同的查询得到彼此独立的新变量。
        Term::Variable(name) => context
            .get(name)
            .map(|scheme| (Substitution::new(), instantiate(scheme, fresh)))
            .ok_or_else(|| TypeError::UnknownVariable(name.clone())),
        Term::Abstraction {
            parameter,
            annotation,
            body,
        } => {
            // 无标注形参先取得新类型变量；函数体返回的替换也必须作用于形参类型。
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
            // 先处理函数，并把所得替换作用于上下文后再处理实参。
            let (function_substitution, function_type) =
                infer_incremental(context, function, fresh)?;
            let argument_context = apply_context(&function_substitution, context);
            let (argument_substitution, argument_type) =
                infer_incremental(&argument_context, argument, fresh)?;

            // X 表示整个应用的结果。实参阶段可能进一步约束函数类型，所以先把
            // argument_substitution 作用于 function_type，再执行合一。
            let result_type = fresh.fresh();
            let application_substitution = unify(vec![(
                apply(&argument_substitution, &function_type),
                Type::Arrow(Box::new(argument_type), Box::new(result_type.clone())),
            )])?;

            // 后得到的替换作用在外层，依次吸收实参和函数阶段获得的信息。
            let substitution = compose(
                &application_substitution,
                &compose(&argument_substitution, &function_substitution),
            );
            // result_type 是本分支刚创建的新鲜变量，前两个替换不可能约束它；
            // 只需应用当前合一所得的替换，就能得到整个应用的结果类型。
            Ok((substitution, apply(&application_substitution, &result_type)))
        }
        Term::Reference(argument) => {
            let (substitution, ty) = infer_incremental(context, argument, fresh)?;
            Ok((substitution, Type::Reference(Box::new(ty))))
        }
        Term::Dereference(argument) => {
            let (substitution, ty) = infer_incremental(context, argument, fresh)?;
            // 解引用要求实参具有 Ref X；合一同时求出所指元素的类型 X。
            let result = fresh.fresh();
            let dereference = unify(vec![(ty, Type::Reference(Box::new(result.clone())))])?;
            Ok((
                compose(&dereference, &substitution),
                apply(&dereference, &result),
            ))
        }
        Term::Assignment(target, value) => {
            // 先处理赋值目标；处理右值前，必须用目标产生的替换更新上下文。
            let (target_substitution, target_type) = infer_incremental(context, target, fresh)?;
            let value_context = apply_context(&target_substitution, context);
            let (value_substitution, value_type) = infer_incremental(&value_context, value, fresh)?;

            // 右值阶段也可能约束目标类型，因此先更新 target_type，再要求它为 Ref T。
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
            // 序列的第一项必须为 Unit；确认后再在更新后的上下文中处理第二项。
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
            // 守卫必须为 Bool；这一检查产生的替换对两个分支都可见。
            let (guard_substitution, guard_type) = infer_incremental(context, guard, fresh)?;
            let guard_check = unify(vec![(guard_type, Type::Bool)])?;
            let after_guard = compose(&guard_check, &guard_substitution);
            let branch_context = apply_context(&after_guard, context);
            let (then_substitution, then_type) =
                infer_incremental(&branch_context, then_term, fresh)?;

            // 后一分支在前一分支更新过的上下文中检查，最后再合一两个分支类型。
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
            // 先把绑定项产生的替换作用于上下文和它自己的类型。
            let (value_substitution, value_type) = infer_incremental(context, value, fresh)?;
            let value_context = apply_context(&value_substitution, context);
            let value_type = apply(&value_substitution, &value_type);

            // 有引用时采用值限制：只有句法值可以泛化，其他项保持单态。
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
/// 使用 ML 风格的 `let` 泛化重构主类型。
pub fn let_principal_type(context: &SchemeContext, term: &Term) -> Result<Type, TypeError> {
    let mut fresh = FreshVariables::default();
    // 生成新变量前，先登记上下文以及整棵项的标注中已经使用的变量名。
    for scheme in context.values() {
        fresh.reserve_type(&scheme.ty);
    }
    fresh.reserve_term_annotations(term);
    // 辅助函数已经把累计替换应用于返回类型；最外层不再需要该替换。
    let (_, ty) = infer_incremental(context, term, &mut fresh)?;
    Ok(ty)
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
            annotation: Some(Type::Variable("X".into())),
            body: Box::new(variable("x")),
        };
        let ty = principal_type(&Context::new(), &identity).unwrap();
        assert!(matches!(ty, Type::Arrow(left, right) if left == right));
    }

    #[test]
    fn reconstructs_application() {
        let apply_to_zero = Term::Abstraction {
            parameter: "f".into(),
            annotation: Some(Type::Variable("F".into())),
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
    fn incremental_reconbase_matches_the_exercise_examples() {
        let identity = Term::Abstraction {
            parameter: "x".into(),
            annotation: Some(Type::Variable("X".into())),
            body: Box::new(variable("x")),
        };
        assert_eq!(
            principal_type_incremental(&Context::new(), &identity).unwrap(),
            Type::Arrow(
                Box::new(Type::Variable("X".into())),
                Box::new(Type::Variable("X".into())),
            )
        );

        let nested_application = Term::Abstraction {
            parameter: "z".into(),
            annotation: Some(Type::Variable("ZZ".into())),
            body: Box::new(Term::Abstraction {
                parameter: "y".into(),
                annotation: Some(Type::Variable("YY".into())),
                body: Box::new(Term::Application(
                    Box::new(variable("z")),
                    Box::new(Term::Application(
                        Box::new(variable("y")),
                        Box::new(Term::True),
                    )),
                )),
            }),
        };
        assert_eq!(
            principal_type_incremental(&Context::new(), &nested_application).unwrap(),
            Type::Arrow(
                Box::new(Type::Arrow(
                    Box::new(Type::Variable("?X0".into())),
                    Box::new(Type::Variable("?X1".into())),
                )),
                Box::new(Type::Arrow(
                    Box::new(Type::Arrow(
                        Box::new(Type::Bool),
                        Box::new(Type::Variable("?X0".into())),
                    )),
                    Box::new(Type::Variable("?X1".into())),
                )),
            )
        );

        let conditional = Term::Abstraction {
            parameter: "w".into(),
            annotation: Some(Type::Variable("W".into())),
            body: Box::new(Term::If(
                Box::new(Term::True),
                Box::new(Term::False),
                Box::new(Term::Application(
                    Box::new(variable("w")),
                    Box::new(Term::False),
                )),
            )),
        };
        assert_eq!(
            principal_type_incremental(&Context::new(), &conditional).unwrap(),
            Type::Arrow(
                Box::new(Type::Arrow(Box::new(Type::Bool), Box::new(Type::Bool))),
                Box::new(Type::Bool),
            )
        );
    }

    #[test]
    fn incremental_reconbase_does_not_accept_later_syntax() {
        let unannotated = Term::Abstraction {
            parameter: "x".into(),
            annotation: None,
            body: Box::new(variable("x")),
        };
        assert!(matches!(
            principal_type_incremental(&Context::new(), &unannotated),
            Err(TypeError::UnsupportedInReconbase(_))
        ));
    }

    #[test]
    fn incremental_helpers_return_types_with_their_substitutions_applied() {
        let annotated_application = Term::Application(
            Box::new(Term::Abstraction {
                parameter: "x".into(),
                annotation: Some(Type::Variable("Y".into())),
                body: Box::new(variable("x")),
            }),
            Box::new(Term::Zero),
        );
        let mut fresh = FreshVariables::default();
        fresh.reserve_term_annotations(&annotated_application);
        let (substitution, ty) =
            infer_reconbase_incremental(&Context::new(), &annotated_application, &mut fresh)
                .unwrap();
        assert_eq!(ty, Type::Nat);
        assert_eq!(apply(&substitution, &ty), ty);

        let let_application = Term::Let {
            name: "id".into(),
            value: Box::new(Term::Abstraction {
                parameter: "x".into(),
                annotation: None,
                body: Box::new(variable("x")),
            }),
            body: Box::new(Term::Application(
                Box::new(variable("id")),
                Box::new(Term::Zero),
            )),
        };
        let mut fresh = FreshVariables::default();
        fresh.reserve_term_annotations(&let_application);
        let (substitution, ty) =
            infer_incremental(&SchemeContext::new(), &let_application, &mut fresh).unwrap();
        assert_eq!(ty, Type::Nat);
        assert_eq!(apply(&substitution, &ty), ty);
    }

    #[test]
    fn let_inference_returns_a_principal_type() {
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
