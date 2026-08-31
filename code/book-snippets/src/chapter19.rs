//! A directly executable Featherweight Java core and a bounded-generic extension.

use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Term {
    Variable(String),
    Field(Box<Term>, String),
    Invoke(Box<Term>, String, Vec<Term>),
    New(String, Vec<Term>),
    Cast(String, Box<Term>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Method {
    pub result: String,
    pub name: String,
    pub parameters: Vec<(String, String)>,
    pub body: Term,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Constructor {
    pub parameters: Vec<(String, String)>,
    pub super_arguments: Vec<String>,
    pub assignments: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Class {
    pub name: String,
    pub superclass: String,
    pub fields: Vec<(String, String)>,
    pub constructor: Constructor,
    pub methods: Vec<Method>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FjError {
    DuplicateClass(String),
    ReservedClass(String),
    UnknownClass(String),
    InheritanceCycle(String),
    DuplicateField(String),
    FieldShadowing(String),
    DuplicateMethod(String),
    DuplicateParameter(String),
    ReservedParameter(String),
    BadConstructor(String),
    BadOverride(String),
    UnknownVariable(String),
    UnknownField { class: String, field: String },
    UnknownMethod { class: String, method: String },
    UnknownLocation(usize),
    WrongArity { expected: usize, actual: usize },
    TypeMismatch { expected: String, actual: String },
    FailedCast { runtime: String, target: String },
    Stuck(Term),
    StepLimit,
}

#[derive(Clone, Debug)]
pub struct ClassTable {
    classes: HashMap<String, Class>,
}

impl ClassTable {
    pub fn new(classes: Vec<Class>) -> Result<Self, FjError> {
        let mut table = HashMap::new();
        for class in classes {
            let name = class.name.clone();
            if name == "Object" {
                return Err(FjError::ReservedClass(name));
            }
            if table.insert(name.clone(), class).is_some() {
                return Err(FjError::DuplicateClass(name));
            }
        }
        let result = Self { classes: table };
        result.check_classes()?;
        Ok(result)
    }

    fn class(&self, name: &str) -> Result<&Class, FjError> {
        self.classes
            .get(name)
            .ok_or_else(|| FjError::UnknownClass(name.to_owned()))
    }

    pub fn is_subtype(&self, source: &str, target: &str) -> bool {
        if source == target {
            return true;
        }
        let mut current = source;
        let mut visited = HashSet::new();
        while current != "Object" && visited.insert(current.to_owned()) {
            let Ok(class) = self.class(current) else {
                return false;
            };
            current = &class.superclass;
            if current == target {
                return true;
            }
        }
        false
    }

    pub fn fields(&self, class_name: &str) -> Result<Vec<(String, String)>, FjError> {
        if class_name == "Object" {
            return Ok(Vec::new());
        }
        let class = self.class(class_name)?;
        let mut fields = self.fields(&class.superclass)?;
        fields.extend(class.fields.clone());
        Ok(fields)
    }

    pub fn method(&self, name: &str, class_name: &str) -> Result<Method, FjError> {
        if class_name == "Object" {
            return Err(FjError::UnknownMethod {
                class: class_name.to_owned(),
                method: name.to_owned(),
            });
        }
        let class = self.class(class_name)?;
        class
            .methods
            .iter()
            .find(|method| method.name == name)
            .cloned()
            .map_or_else(|| self.method(name, &class.superclass), Ok)
    }

    fn check_classes(&self) -> Result<(), FjError> {
        for class in self.classes.values() {
            if class.superclass != "Object" && !self.classes.contains_key(&class.superclass) {
                return Err(FjError::UnknownClass(class.superclass.clone()));
            }
            let mut current = class.name.as_str();
            let mut visited = HashSet::new();
            while current != "Object" {
                if !visited.insert(current.to_owned()) {
                    return Err(FjError::InheritanceCycle(class.name.clone()));
                }
                current = &self.class(current)?.superclass;
            }

            let inherited = self.fields(&class.superclass)?;
            for (field_type, _) in inherited.iter().chain(class.fields.iter()) {
                if field_type != "Object" && !self.classes.contains_key(field_type) {
                    return Err(FjError::UnknownClass(field_type.clone()));
                }
            }
            let mut field_names: HashSet<String> =
                inherited.iter().map(|(_, name)| name.clone()).collect();
            for (_, name) in &class.fields {
                if !field_names.insert(name.clone()) {
                    return Err(FjError::FieldShadowing(name.clone()));
                }
            }
            let own_names: HashSet<_> = class.fields.iter().map(|(_, name)| name).collect();
            if own_names.len() != class.fields.len() {
                return Err(FjError::DuplicateField(class.name.clone()));
            }

            let expected_parameters: Vec<_> = inherited
                .iter()
                .chain(class.fields.iter())
                .cloned()
                .collect();
            let inherited_names: Vec<_> = inherited.iter().map(|(_, name)| name.clone()).collect();
            let own_names: Vec<_> = class.fields.iter().map(|(_, name)| name.clone()).collect();
            if class.constructor.parameters != expected_parameters
                || class.constructor.super_arguments != inherited_names
                || class.constructor.assignments != own_names
            {
                return Err(FjError::BadConstructor(class.name.clone()));
            }

            let mut method_names = HashSet::new();
            for method in &class.methods {
                if !method_names.insert(method.name.clone()) {
                    return Err(FjError::DuplicateMethod(method.name.clone()));
                }
                if method.result != "Object" && !self.classes.contains_key(&method.result) {
                    return Err(FjError::UnknownClass(method.result.clone()));
                }
                let mut parameter_names = HashSet::new();
                for (parameter_type, parameter_name) in &method.parameters {
                    if parameter_type != "Object" && !self.classes.contains_key(parameter_type) {
                        return Err(FjError::UnknownClass(parameter_type.clone()));
                    }
                    if parameter_name == "this" {
                        return Err(FjError::ReservedParameter(method.name.clone()));
                    }
                    if !parameter_names.insert(parameter_name.clone()) {
                        return Err(FjError::DuplicateParameter(method.name.clone()));
                    }
                }
                if let Ok(super_method) = self.method(&method.name, &class.superclass)
                    && (method.result != super_method.result
                        || method
                            .parameters
                            .iter()
                            .map(|(ty, _)| ty)
                            .ne(super_method.parameters.iter().map(|(ty, _)| ty)))
                {
                    return Err(FjError::BadOverride(method.name.clone()));
                }
                let mut context: HashMap<String, String> = method
                    .parameters
                    .iter()
                    .map(|(ty, name)| (name.clone(), ty.clone()))
                    .collect();
                context.insert("this".to_owned(), class.name.clone());
                let body_type = self.type_of(&context, &method.body)?.class;
                if !self.is_subtype(&body_type, &method.result) {
                    return Err(FjError::TypeMismatch {
                        expected: method.result.clone(),
                        actual: body_type,
                    });
                }
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CastWarning {
    StupidCast { source: String, target: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypedTerm {
    pub class: String,
    pub warnings: Vec<CastWarning>,
}

impl ClassTable {
    pub fn type_of(
        &self,
        context: &HashMap<String, String>,
        term: &Term,
    ) -> Result<TypedTerm, FjError> {
        match term {
            Term::Variable(name) => context
                .get(name)
                .cloned()
                .map(|class| TypedTerm {
                    class,
                    warnings: Vec::new(),
                })
                .ok_or_else(|| FjError::UnknownVariable(name.clone())),
            Term::Field(receiver, field) => {
                let receiver_type = self.type_of(context, receiver)?;
                let class = self
                    .fields(&receiver_type.class)?
                    .into_iter()
                    .find(|(_, name)| name == field)
                    .map(|(class, _)| class)
                    .ok_or_else(|| FjError::UnknownField {
                        class: receiver_type.class.clone(),
                        field: field.clone(),
                    })?;
                Ok(TypedTerm {
                    class,
                    warnings: receiver_type.warnings,
                })
            }
            Term::Invoke(receiver, method_name, arguments) => {
                let mut receiver_type = self.type_of(context, receiver)?;
                let method = self.method(method_name, &receiver_type.class)?;
                if method.parameters.len() != arguments.len() {
                    return Err(FjError::WrongArity {
                        expected: method.parameters.len(),
                        actual: arguments.len(),
                    });
                }
                for ((expected, _), argument) in method.parameters.iter().zip(arguments) {
                    let mut actual = self.type_of(context, argument)?;
                    if !self.is_subtype(&actual.class, expected) {
                        return Err(FjError::TypeMismatch {
                            expected: expected.clone(),
                            actual: actual.class,
                        });
                    }
                    receiver_type.warnings.append(&mut actual.warnings);
                }
                receiver_type.class = method.result;
                Ok(receiver_type)
            }
            Term::New(class, arguments) => {
                let fields = self.fields(class)?;
                if fields.len() != arguments.len() {
                    return Err(FjError::WrongArity {
                        expected: fields.len(),
                        actual: arguments.len(),
                    });
                }
                let mut warnings = Vec::new();
                for ((expected, _), argument) in fields.iter().zip(arguments) {
                    let mut actual = self.type_of(context, argument)?;
                    if !self.is_subtype(&actual.class, expected) {
                        return Err(FjError::TypeMismatch {
                            expected: expected.clone(),
                            actual: actual.class,
                        });
                    }
                    warnings.append(&mut actual.warnings);
                }
                Ok(TypedTerm {
                    class: class.clone(),
                    warnings,
                })
            }
            Term::Cast(target, subject) => {
                let mut subject_type = self.type_of(context, subject)?;
                if !self.is_subtype(&subject_type.class, target)
                    && !self.is_subtype(target, &subject_type.class)
                {
                    subject_type.warnings.push(CastWarning::StupidCast {
                        source: subject_type.class.clone(),
                        target: target.clone(),
                    });
                }
                subject_type.class.clone_from(target);
                Ok(subject_type)
            }
        }
    }
}

fn is_value(term: &Term) -> bool {
    matches!(term, Term::New(_, arguments) if arguments.iter().all(is_value))
}

fn substitute(term: &Term, values: &HashMap<String, Term>) -> Term {
    match term {
        Term::Variable(name) => values.get(name).cloned().unwrap_or_else(|| term.clone()),
        Term::Field(receiver, field) => {
            Term::Field(Box::new(substitute(receiver, values)), field.clone())
        }
        Term::Invoke(receiver, method, arguments) => Term::Invoke(
            Box::new(substitute(receiver, values)),
            method.clone(),
            arguments
                .iter()
                .map(|argument| substitute(argument, values))
                .collect(),
        ),
        Term::New(class, arguments) => Term::New(
            class.clone(),
            arguments
                .iter()
                .map(|argument| substitute(argument, values))
                .collect(),
        ),
        Term::Cast(target, subject) => {
            Term::Cast(target.clone(), Box::new(substitute(subject, values)))
        }
    }
}

impl ClassTable {
    pub fn eval1(&self, term: &Term) -> Result<Option<Term>, FjError> {
        match term {
            Term::Field(receiver, field) => {
                if let Term::New(class, arguments) = receiver.as_ref()
                    && arguments.iter().all(is_value)
                {
                    let fields = self.fields(class)?;
                    return fields
                        .iter()
                        .position(|(_, name)| name == field)
                        .map(|index| arguments[index].clone())
                        .ok_or_else(|| FjError::UnknownField {
                            class: class.clone(),
                            field: field.clone(),
                        })
                        .map(Some);
                }
                self.eval1(receiver)
                    .map(|step| step.map(|next| Term::Field(Box::new(next), field.clone())))
            }
            Term::Invoke(receiver, method_name, arguments) => {
                if !is_value(receiver) {
                    return self.eval1(receiver).map(|step| {
                        step.map(|next| {
                            Term::Invoke(Box::new(next), method_name.clone(), arguments.clone())
                        })
                    });
                }
                if let Some((index, argument)) = arguments
                    .iter()
                    .enumerate()
                    .find(|(_, term)| !is_value(term))
                {
                    return self.eval1(argument).map(|step| {
                        step.map(|next| {
                            let mut updated = arguments.clone();
                            updated[index] = next;
                            Term::Invoke(receiver.clone(), method_name.clone(), updated)
                        })
                    });
                }
                if let Term::New(class, _) = receiver.as_ref() {
                    let method = self.method(method_name, class)?;
                    if method.parameters.len() != arguments.len() {
                        return Err(FjError::WrongArity {
                            expected: method.parameters.len(),
                            actual: arguments.len(),
                        });
                    }
                    let mut values: HashMap<String, Term> = method
                        .parameters
                        .iter()
                        .map(|(_, name)| name.clone())
                        .zip(arguments.iter().cloned())
                        .collect();
                    values.insert("this".to_owned(), receiver.as_ref().clone());
                    return Ok(Some(substitute(&method.body, &values)));
                }
                Ok(None)
            }
            Term::New(class, arguments) => {
                if let Some((index, argument)) = arguments
                    .iter()
                    .enumerate()
                    .find(|(_, term)| !is_value(term))
                {
                    self.eval1(argument).map(|step| {
                        step.map(|next| {
                            let mut updated = arguments.clone();
                            updated[index] = next;
                            Term::New(class.clone(), updated)
                        })
                    })
                } else {
                    Ok(None)
                }
            }
            Term::Cast(target, subject) => {
                if !is_value(subject) {
                    return self
                        .eval1(subject)
                        .map(|step| step.map(|next| Term::Cast(target.clone(), Box::new(next))));
                }
                if let Term::New(runtime, _) = subject.as_ref() {
                    if self.is_subtype(runtime, target) {
                        Ok(Some(subject.as_ref().clone()))
                    } else {
                        Err(FjError::FailedCast {
                            runtime: runtime.clone(),
                            target: target.clone(),
                        })
                    }
                } else {
                    Ok(None)
                }
            }
            Term::Variable(_) => Ok(None),
        }
    }

    pub fn eval(&self, mut term: Term, limit: usize) -> Result<Term, FjError> {
        for _ in 0..limit {
            match self.eval1(&term)? {
                Some(next) => term = next,
                None if is_value(&term) => return Ok(term),
                None => return Err(FjError::Stuck(term)),
            }
        }
        Err(FjError::StepLimit)
    }
}

pub type Store = HashMap<usize, Term>;

/// 实现 E-CastStore 的关键检查：位置的静态类名不足以决定转换结果；
/// 转换还必须检查该位置所存对象的运行时类。
pub fn eval_location_cast(
    table: &ClassTable,
    store: &Store,
    target: &str,
    location: usize,
) -> Result<usize, FjError> {
    let object = store
        .get(&location)
        .ok_or(FjError::UnknownLocation(location))?;
    let Term::New(runtime, arguments) = object else {
        return Err(FjError::Stuck(object.clone()));
    };
    if !arguments.iter().all(is_value) {
        return Err(FjError::Stuck(object.clone()));
    }
    if table.is_subtype(runtime, target) {
        Ok(location)
    } else {
        Err(FjError::FailedCast {
            runtime: runtime.clone(),
            target: target.to_owned(),
        })
    }
}

// TAPL-SNIPPET-BEGIN: ch19-gj-syntax
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GType {
    Variable(String),
    Class(String, Vec<GType>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TypeParameter {
    pub name: String,
    pub bound: GType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GTerm {
    Variable(String),
    Field(Box<GTerm>, String),
    Invoke(Box<GTerm>, String, Vec<GType>, Vec<GTerm>),
    New(GType, Vec<GTerm>),
    Cast(GType, Box<GTerm>),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GMethod {
    pub type_parameters: Vec<TypeParameter>,
    pub result: GType,
    pub name: String,
    pub parameters: Vec<(GType, String)>,
    pub body: GTerm,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GClass {
    pub name: String,
    pub type_parameters: Vec<TypeParameter>,
    pub superclass: GType,
    pub fields: Vec<(GType, String)>,
    pub methods: Vec<GMethod>,
}
// TAPL-SNIPPET-END: ch19-gj-syntax

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GjError {
    DuplicateClass(String),
    ReservedClass(String),
    DuplicateTypeParameter(String),
    DuplicateField(String),
    DuplicateMethod(String),
    DuplicateParameter(String),
    ReservedParameter(String),
    InheritanceCycle(String),
    UnknownType(String),
    UnknownVariable(String),
    UnknownField(String),
    UnknownMethod(String),
    WrongTypeArity { expected: usize, actual: usize },
    WrongTermArity { expected: usize, actual: usize },
    BoundViolation { argument: GType, bound: GType },
    TypeMismatch { expected: GType, actual: GType },
    BadOverride(String),
    ErasedProgram(FjError),
}

#[derive(Clone, Debug)]
pub struct GenericTable {
    classes: HashMap<String, GClass>,
}

// TAPL-SNIPPET-BEGIN: ch19-gj-type-substitution
fn substitute_type(ty: &GType, substitution: &HashMap<String, GType>) -> GType {
    match ty {
        GType::Variable(name) => substitution
            .get(name)
            .cloned()
            .unwrap_or_else(|| ty.clone()),
        GType::Class(name, arguments) => GType::Class(
            name.clone(),
            arguments
                .iter()
                .map(|argument| substitute_type(argument, substitution))
                .collect(),
        ),
    }
}
// TAPL-SNIPPET-END: ch19-gj-type-substitution

fn normalized_method_signature(method: &GMethod) -> (Vec<GType>, GType, Vec<GType>) {
    // 这些内部名称不会与下面比较中的源语言标识符混淆；
    // 它们为两份方法签名采用相同的 alpha 重命名。
    let substitution: HashMap<_, _> = method
        .type_parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            (
                parameter.name.clone(),
                GType::Variable(format!("#method-{index}")),
            )
        })
        .collect();
    let bounds = method
        .type_parameters
        .iter()
        .map(|parameter| substitute_type(&parameter.bound, &substitution))
        .collect();
    let result = substitute_type(&method.result, &substitution);
    let parameters = method
        .parameters
        .iter()
        .map(|(ty, _)| substitute_type(ty, &substitution))
        .collect();
    (bounds, result, parameters)
}

fn same_method_signature(method: &GMethod, inherited: &GMethod) -> bool {
    method.type_parameters.len() == inherited.type_parameters.len()
        && normalized_method_signature(method) == normalized_method_signature(inherited)
}

impl GenericTable {
    fn erased_fields(&self, class_name: &str) -> Result<Vec<(String, String)>, GjError> {
        if class_name == "Object" {
            return Ok(Vec::new());
        }
        let class = self.class(class_name)?;
        let bounds = self.extend_bounds(&HashMap::new(), &class.type_parameters)?;
        let superclass = erase_type(&class.superclass, &bounds);
        let mut fields = self.erased_fields(&superclass)?;
        fields.extend(
            class
                .fields
                .iter()
                .map(|(ty, name)| (erase_type(ty, &bounds), name.clone())),
        );
        Ok(fields)
    }

    pub fn new(classes: Vec<GClass>) -> Result<Self, GjError> {
        let mut table = HashMap::new();
        for class in classes {
            let name = class.name.clone();
            if name == "Object" {
                return Err(GjError::ReservedClass(name));
            }
            if table.insert(name.clone(), class).is_some() {
                return Err(GjError::DuplicateClass(name));
            }
        }
        let result = Self { classes: table };
        result.check_classes()?;
        Ok(result)
    }

    fn class(&self, name: &str) -> Result<&GClass, GjError> {
        self.classes
            .get(name)
            .ok_or_else(|| GjError::UnknownType(name.to_owned()))
    }

    // TAPL-SNIPPET-BEGIN: ch19-gj-subtyping
    pub fn is_subtype(
        &self,
        source: &GType,
        target: &GType,
        bounds: &HashMap<String, GType>,
    ) -> bool {
        self.is_subtype_avoiding_cycles(source, target, bounds, &mut HashSet::new())
    }

    fn is_subtype_avoiding_cycles(
        &self,
        source: &GType,
        target: &GType,
        bounds: &HashMap<String, GType>,
        visited: &mut HashSet<GType>,
    ) -> bool {
        if source == target {
            return true;
        }
        if !visited.insert(source.clone()) {
            return false;
        }
        match source {
            GType::Variable(name) => bounds.get(name).is_some_and(|bound| {
                self.is_subtype_avoiding_cycles(bound, target, bounds, visited)
            }),
            GType::Class(name, arguments) if name != "Object" => {
                let Ok(class) = self.class(name) else {
                    return false;
                };
                let substitution: HashMap<_, _> = class
                    .type_parameters
                    .iter()
                    .map(|parameter| parameter.name.clone())
                    .zip(arguments.iter().cloned())
                    .collect();
                self.is_subtype_avoiding_cycles(
                    &substitute_type(&class.superclass, &substitution),
                    target,
                    bounds,
                    visited,
                )
            }
            GType::Class(_, _) => false,
        }
    }

    pub fn check_type(&self, ty: &GType, bounds: &HashMap<String, GType>) -> Result<(), GjError> {
        match ty {
            GType::Variable(name) => bounds
                .contains_key(name)
                .then_some(())
                .ok_or_else(|| GjError::UnknownType(name.clone())),
            GType::Class(name, arguments) if name == "Object" && arguments.is_empty() => Ok(()),
            GType::Class(name, arguments) => {
                let class = self.class(name)?;
                if class.type_parameters.len() != arguments.len() {
                    return Err(GjError::WrongTypeArity {
                        expected: class.type_parameters.len(),
                        actual: arguments.len(),
                    });
                }
                for argument in arguments {
                    self.check_type(argument, bounds)?;
                }
                let substitution: HashMap<_, _> = class
                    .type_parameters
                    .iter()
                    .map(|parameter| parameter.name.clone())
                    .zip(arguments.iter().cloned())
                    .collect();
                for (parameter, argument) in class.type_parameters.iter().zip(arguments) {
                    let bound = substitute_type(&parameter.bound, &substitution);
                    if !self.is_subtype(argument, &bound, bounds) {
                        return Err(GjError::BoundViolation {
                            argument: argument.clone(),
                            bound,
                        });
                    }
                }
                Ok(())
            }
        }
    }
    // TAPL-SNIPPET-END: ch19-gj-subtyping

    fn extend_bounds(
        &self,
        outer: &HashMap<String, GType>,
        parameters: &[TypeParameter],
    ) -> Result<HashMap<String, GType>, GjError> {
        let mut result = outer.clone();
        for parameter in parameters {
            if result
                .insert(parameter.name.clone(), parameter.bound.clone())
                .is_some()
            {
                return Err(GjError::DuplicateTypeParameter(parameter.name.clone()));
            }
        }
        for parameter in parameters {
            self.check_type(&parameter.bound, &result)?;
        }
        Ok(result)
    }

    fn check_classes(&self) -> Result<(), GjError> {
        for class in self.classes.values() {
            let bounds = self.extend_bounds(&HashMap::new(), &class.type_parameters)?;
            self.check_type(&class.superclass, &bounds)?;

            let mut current = GType::Class(
                class.name.clone(),
                class
                    .type_parameters
                    .iter()
                    .map(|parameter| GType::Variable(parameter.name.clone()))
                    .collect(),
            );
            let mut visited = HashSet::new();
            loop {
                let GType::Class(name, arguments) = current else {
                    return Err(GjError::UnknownType(format!("{:?}", class.superclass)));
                };
                if name == "Object" {
                    break;
                }
                if !visited.insert(name.clone()) {
                    return Err(GjError::InheritanceCycle(class.name.clone()));
                }
                let current_class = self.class(&name)?;
                let substitution: HashMap<_, _> = current_class
                    .type_parameters
                    .iter()
                    .map(|parameter| parameter.name.clone())
                    .zip(arguments)
                    .collect();
                current = substitute_type(&current_class.superclass, &substitution);
            }

            let inherited = self.fields(&class.superclass, &bounds)?;
            let mut field_names: HashSet<_> =
                inherited.iter().map(|(_, name)| name.clone()).collect();
            for (field_type, field_name) in &class.fields {
                self.check_type(field_type, &bounds)?;
                if !field_names.insert(field_name.clone()) {
                    return Err(GjError::DuplicateField(field_name.clone()));
                }
            }

            let this_type = GType::Class(
                class.name.clone(),
                class
                    .type_parameters
                    .iter()
                    .map(|parameter| GType::Variable(parameter.name.clone()))
                    .collect(),
            );
            let mut method_names = HashSet::new();
            for method in &class.methods {
                if !method_names.insert(method.name.clone()) {
                    return Err(GjError::DuplicateMethod(method.name.clone()));
                }
                let method_bounds = self.extend_bounds(&bounds, &method.type_parameters)?;
                self.check_type(&method.result, &method_bounds)?;
                let mut parameter_names = HashSet::new();
                let mut context = HashMap::new();
                for (parameter_type, parameter_name) in &method.parameters {
                    self.check_type(parameter_type, &method_bounds)?;
                    if parameter_name == "this" {
                        return Err(GjError::ReservedParameter(method.name.clone()));
                    }
                    if !parameter_names.insert(parameter_name.clone()) {
                        return Err(GjError::DuplicateParameter(method.name.clone()));
                    }
                    context.insert(parameter_name.clone(), parameter_type.clone());
                }
                context.insert("this".to_owned(), this_type.clone());
                let body_type = self.type_of(&method_bounds, &context, &method.body)?;
                if !self.is_subtype(&body_type, &method.result, &method_bounds) {
                    return Err(GjError::TypeMismatch {
                        expected: method.result.clone(),
                        actual: body_type,
                    });
                }

                match self.method(&method.name, &class.superclass, &bounds) {
                    Ok(super_method) if !same_method_signature(method, &super_method) => {
                        return Err(GjError::BadOverride(method.name.clone()));
                    }
                    Ok(_) | Err(GjError::UnknownMethod(_)) => {}
                    Err(error) => return Err(error),
                }
            }
        }
        Ok(())
    }

    fn fields(
        &self,
        ty: &GType,
        bounds: &HashMap<String, GType>,
    ) -> Result<Vec<(GType, String)>, GjError> {
        if let GType::Variable(name) = ty {
            let bound = bounds
                .get(name)
                .ok_or_else(|| GjError::UnknownType(name.clone()))?;
            return self.fields(bound, bounds);
        }
        let GType::Class(name, arguments) = ty else {
            unreachable!("all GType variants handled")
        };
        if name == "Object" {
            return Ok(Vec::new());
        }
        let class = self.class(name)?;
        let substitution: HashMap<_, _> = class
            .type_parameters
            .iter()
            .map(|parameter| parameter.name.clone())
            .zip(arguments.iter().cloned())
            .collect();
        let superclass = substitute_type(&class.superclass, &substitution);
        let mut fields = self.fields(&superclass, bounds)?;
        fields.extend(class.fields.iter().map(|(field_type, field_name)| {
            (
                substitute_type(field_type, &substitution),
                field_name.clone(),
            )
        }));
        Ok(fields)
    }

    fn method(
        &self,
        name: &str,
        receiver: &GType,
        bounds: &HashMap<String, GType>,
    ) -> Result<GMethod, GjError> {
        if let GType::Variable(variable) = receiver {
            let bound = bounds
                .get(variable)
                .ok_or_else(|| GjError::UnknownType(variable.clone()))?;
            return self.method(name, bound, bounds);
        }
        let GType::Class(class_name, arguments) = receiver else {
            unreachable!("all GType variants handled")
        };
        if class_name == "Object" {
            return Err(GjError::UnknownMethod(name.to_owned()));
        }
        let class = self.class(class_name)?;
        let substitution: HashMap<_, _> = class
            .type_parameters
            .iter()
            .map(|parameter| parameter.name.clone())
            .zip(arguments.iter().cloned())
            .collect();
        if let Some(method) = class.methods.iter().find(|method| method.name == name) {
            let mut result = method.clone();
            result.result = substitute_type(&result.result, &substitution);
            result.parameters = result
                .parameters
                .into_iter()
                .map(|(ty, parameter)| (substitute_type(&ty, &substitution), parameter))
                .collect();
            result.type_parameters = result
                .type_parameters
                .into_iter()
                .map(|parameter| TypeParameter {
                    name: parameter.name,
                    bound: substitute_type(&parameter.bound, &substitution),
                })
                .collect();
            Ok(result)
        } else {
            self.method(
                name,
                &substitute_type(&class.superclass, &substitution),
                bounds,
            )
        }
    }

    pub fn type_of(
        &self,
        bounds: &HashMap<String, GType>,
        context: &HashMap<String, GType>,
        term: &GTerm,
    ) -> Result<GType, GjError> {
        match term {
            GTerm::Variable(name) => context
                .get(name)
                .cloned()
                .ok_or_else(|| GjError::UnknownVariable(name.clone())),
            GTerm::Field(receiver, field) => {
                let receiver_type = self.type_of(bounds, context, receiver)?;
                self.fields(&receiver_type, bounds)?
                    .into_iter()
                    .find(|(_, name)| name == field)
                    .map(|(ty, _)| ty)
                    .ok_or_else(|| GjError::UnknownField(field.clone()))
            }
            GTerm::New(class_type, arguments) => {
                self.check_type(class_type, bounds)?;
                let fields = self.fields(class_type, bounds)?;
                self.check_arguments(bounds, context, &fields, arguments)?;
                Ok(class_type.clone())
            }
            GTerm::Cast(target, subject) => {
                self.check_type(target, bounds)?;
                let _ = self.type_of(bounds, context, subject)?;
                Ok(target.clone())
            }
            GTerm::Invoke(receiver, name, type_arguments, arguments) => {
                let receiver_type = self.type_of(bounds, context, receiver)?;
                let method = self.method(name, &receiver_type, bounds)?;
                if method.type_parameters.len() != type_arguments.len() {
                    return Err(GjError::WrongTypeArity {
                        expected: method.type_parameters.len(),
                        actual: type_arguments.len(),
                    });
                }
                let substitution: HashMap<_, _> = method
                    .type_parameters
                    .iter()
                    .map(|parameter| parameter.name.clone())
                    .zip(type_arguments.iter().cloned())
                    .collect();
                for (parameter, argument) in method.type_parameters.iter().zip(type_arguments) {
                    self.check_type(argument, bounds)?;
                    let bound = substitute_type(&parameter.bound, &substitution);
                    if !self.is_subtype(argument, &bound, bounds) {
                        return Err(GjError::BoundViolation {
                            argument: argument.clone(),
                            bound,
                        });
                    }
                }
                let parameters: Vec<_> = method
                    .parameters
                    .iter()
                    .map(|(ty, name)| (substitute_type(ty, &substitution), name.clone()))
                    .collect();
                self.check_arguments(bounds, context, &parameters, arguments)?;
                Ok(substitute_type(&method.result, &substitution))
            }
        }
    }

    fn check_arguments(
        &self,
        bounds: &HashMap<String, GType>,
        context: &HashMap<String, GType>,
        parameters: &[(GType, String)],
        arguments: &[GTerm],
    ) -> Result<(), GjError> {
        if parameters.len() != arguments.len() {
            return Err(GjError::WrongTermArity {
                expected: parameters.len(),
                actual: arguments.len(),
            });
        }
        for ((expected, _), argument) in parameters.iter().zip(arguments) {
            let actual = self.type_of(bounds, context, argument)?;
            if !self.is_subtype(&actual, expected, bounds) {
                return Err(GjError::TypeMismatch {
                    expected: expected.clone(),
                    actual,
                });
            }
        }
        Ok(())
    }
}

pub fn erase_type<S: std::hash::BuildHasher>(
    ty: &GType,
    bounds: &HashMap<String, GType, S>,
) -> String {
    match ty {
        GType::Variable(name) => bounds
            .get(name)
            .map_or_else(|| "Object".to_owned(), |bound| erase_type(bound, bounds)),
        GType::Class(name, _) => name.clone(),
    }
}

impl GenericTable {
    fn erased_field_type(
        &self,
        receiver: &GType,
        field: &str,
        bounds: &HashMap<String, GType>,
    ) -> Result<String, GjError> {
        self.erased_fields(&erase_type(receiver, bounds))?
            .into_iter()
            .find(|(_, name)| name == field)
            .map(|(ty, _)| ty)
            .ok_or_else(|| GjError::UnknownField(field.to_owned()))
    }

    fn erased_method_result(&self, class_name: &str, name: &str) -> Result<String, GjError> {
        if class_name == "Object" {
            return Err(GjError::UnknownMethod(name.to_owned()));
        }
        let class = self.class(class_name)?;
        let class_bounds = self.extend_bounds(&HashMap::new(), &class.type_parameters)?;
        if let Some(method) = class.methods.iter().find(|method| method.name == name) {
            let method_bounds = self.extend_bounds(&class_bounds, &method.type_parameters)?;
            Ok(erase_type(&method.result, &method_bounds))
        } else {
            self.erased_method_result(&erase_type(&class.superclass, &class_bounds), name)
        }
    }

    // TAPL-SNIPPET-BEGIN: ch19-gj-erasure-term
    fn insert_erasure_cast(term: Term, actual: &str, expected: String) -> Term {
        if actual == expected {
            term
        } else {
            Term::Cast(expected, Box::new(term))
        }
    }

    /// 擦除一个良类型 GJ 项；若泛型字段或方法结果的擦除声明更一般，
    /// 则插入 FJ 所需的类型转换。
    fn erase_typed_term(
        &self,
        bounds: &HashMap<String, GType>,
        context: &HashMap<String, GType>,
        term: &GTerm,
    ) -> Result<(Term, GType), GjError> {
        let source_type = self.type_of(bounds, context, term)?;
        let erased = match term {
            GTerm::Variable(name) => Term::Variable(name.clone()),
            GTerm::Field(receiver, field) => {
                let receiver_type = self.type_of(bounds, context, receiver)?;
                let (erased_receiver, _) = self.erase_typed_term(bounds, context, receiver)?;
                let raw_type = self.erased_field_type(&receiver_type, field, bounds)?;
                let raw = Term::Field(Box::new(erased_receiver), field.clone());
                Self::insert_erasure_cast(raw, &raw_type, erase_type(&source_type, bounds))
            }
            GTerm::Invoke(receiver, name, _, arguments) => {
                let receiver_type = self.type_of(bounds, context, receiver)?;
                let (erased_receiver, _) = self.erase_typed_term(bounds, context, receiver)?;
                let erased_arguments = arguments
                    .iter()
                    .map(|argument| {
                        self.erase_typed_term(bounds, context, argument)
                            .map(|(erased, _)| erased)
                    })
                    .collect::<Result<_, _>>()?;
                let raw_type =
                    self.erased_method_result(&erase_type(&receiver_type, bounds), name)?;
                let raw = Term::Invoke(Box::new(erased_receiver), name.clone(), erased_arguments);
                Self::insert_erasure_cast(raw, &raw_type, erase_type(&source_type, bounds))
            }
            GTerm::New(ty, arguments) => Term::New(
                erase_type(ty, bounds),
                arguments
                    .iter()
                    .map(|argument| {
                        self.erase_typed_term(bounds, context, argument)
                            .map(|(erased, _)| erased)
                    })
                    .collect::<Result<_, _>>()?,
            ),
            GTerm::Cast(ty, subject) => Term::Cast(
                erase_type(ty, bounds),
                Box::new(self.erase_typed_term(bounds, context, subject)?.0),
            ),
        };
        Ok((erased, source_type))
    }
    // TAPL-SNIPPET-END: ch19-gj-erasure-term

    fn erase_method(
        &self,
        class: &GClass,
        method: &GMethod,
        class_bounds: &HashMap<String, GType>,
    ) -> Result<Method, GjError> {
        let bounds = self.extend_bounds(class_bounds, &method.type_parameters)?;
        let mut context: HashMap<_, _> = method
            .parameters
            .iter()
            .map(|(ty, name)| (name.clone(), ty.clone()))
            .collect();
        context.insert(
            "this".to_owned(),
            GType::Class(
                class.name.clone(),
                class
                    .type_parameters
                    .iter()
                    .map(|parameter| GType::Variable(parameter.name.clone()))
                    .collect(),
            ),
        );
        Ok(Method {
            result: erase_type(&method.result, &bounds),
            name: method.name.clone(),
            parameters: method
                .parameters
                .iter()
                .map(|(ty, name)| (erase_type(ty, &bounds), name.clone()))
                .collect(),
            body: self.erase_typed_term(&bounds, &context, &method.body)?.0,
        })
    }

    /// 把包括方法体在内的全部泛型声明擦除为 FJ 类表，
    /// 供上面的可执行求值器使用。
    pub fn erase_table(&self) -> Result<ClassTable, GjError> {
        let mut erased = Vec::new();
        for class in self.classes.values() {
            let bounds = self.extend_bounds(&HashMap::new(), &class.type_parameters)?;
            let superclass = erase_type(&class.superclass, &bounds);
            let inherited_fields = self.erased_fields(&superclass)?;
            let own_fields: Vec<_> = class
                .fields
                .iter()
                .map(|(ty, name)| (erase_type(ty, &bounds), name.clone()))
                .collect();
            let parameters = inherited_fields
                .iter()
                .chain(own_fields.iter())
                .cloned()
                .collect();
            erased.push(Class {
                name: class.name.clone(),
                superclass,
                fields: own_fields.clone(),
                constructor: Constructor {
                    parameters,
                    super_arguments: inherited_fields
                        .iter()
                        .map(|(_, name)| name.clone())
                        .collect(),
                    assignments: own_fields.iter().map(|(_, name)| name.clone()).collect(),
                },
                methods: class
                    .methods
                    .iter()
                    .map(|method| self.erase_method(class, method, &bounds))
                    .collect::<Result<_, _>>()?,
            });
        }
        ClassTable::new(erased).map_err(GjError::ErasedProgram)
    }

    /// 对泛型项进行类型检查和擦除，再用 FJ 求值器执行所得项；
    /// 这就是 GJ 层的运行时实现。
    pub fn eval_erased(
        &self,
        bounds: &HashMap<String, GType>,
        context: &HashMap<String, GType>,
        term: &GTerm,
        limit: usize,
    ) -> Result<Term, GjError> {
        let erased = self.erase_typed_term(bounds, context, term)?.0;
        let table = self.erase_table()?;
        table.eval(erased, limit).map_err(GjError::ErasedProgram)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn object_type() -> GType {
        GType::Class("Object".to_owned(), Vec::new())
    }

    fn empty_constructor() -> Constructor {
        Constructor {
            parameters: Vec::new(),
            super_arguments: Vec::new(),
            assignments: Vec::new(),
        }
    }

    fn pair_table() -> ClassTable {
        let a = Class {
            name: "A".to_owned(),
            superclass: "Object".to_owned(),
            fields: Vec::new(),
            constructor: empty_constructor(),
            methods: Vec::new(),
        };
        let b = Class {
            name: "B".to_owned(),
            superclass: "Object".to_owned(),
            fields: Vec::new(),
            constructor: empty_constructor(),
            methods: Vec::new(),
        };
        let pair = Class {
            name: "Pair".to_owned(),
            superclass: "Object".to_owned(),
            fields: vec![
                ("Object".to_owned(), "fst".to_owned()),
                ("Object".to_owned(), "snd".to_owned()),
            ],
            constructor: Constructor {
                parameters: vec![
                    ("Object".to_owned(), "fst".to_owned()),
                    ("Object".to_owned(), "snd".to_owned()),
                ],
                super_arguments: Vec::new(),
                assignments: vec!["fst".to_owned(), "snd".to_owned()],
            },
            methods: vec![Method {
                result: "Pair".to_owned(),
                name: "setfst".to_owned(),
                parameters: vec![("Object".to_owned(), "newfst".to_owned())],
                body: Term::New(
                    "Pair".to_owned(),
                    vec![
                        Term::Variable("newfst".to_owned()),
                        Term::Field(
                            Box::new(Term::Variable("this".to_owned())),
                            "snd".to_owned(),
                        ),
                    ],
                ),
            }],
        };
        ClassTable::new(vec![a, b, pair]).expect("valid Pair class table")
    }

    fn generic_table() -> GenericTable {
        let a = GClass {
            name: "A".to_owned(),
            type_parameters: Vec::new(),
            superclass: object_type(),
            fields: Vec::new(),
            methods: vec![GMethod {
                type_parameters: Vec::new(),
                result: GType::Class("A".to_owned(), Vec::new()),
                name: "self".to_owned(),
                parameters: Vec::new(),
                body: GTerm::Variable("this".to_owned()),
            }],
        };
        let b = GClass {
            name: "B".to_owned(),
            type_parameters: Vec::new(),
            superclass: object_type(),
            fields: Vec::new(),
            methods: Vec::new(),
        };
        let box_class = GClass {
            name: "Box".to_owned(),
            type_parameters: vec![TypeParameter {
                name: "X".to_owned(),
                bound: object_type(),
            }],
            superclass: object_type(),
            fields: vec![(GType::Variable("X".to_owned()), "value".to_owned())],
            methods: vec![GMethod {
                type_parameters: Vec::new(),
                result: GType::Variable("X".to_owned()),
                name: "get".to_owned(),
                parameters: Vec::new(),
                body: GTerm::Field(
                    Box::new(GTerm::Variable("this".to_owned())),
                    "value".to_owned(),
                ),
            }],
        };
        let holder = GClass {
            name: "Holder".to_owned(),
            type_parameters: vec![TypeParameter {
                name: "X".to_owned(),
                bound: GType::Class("A".to_owned(), Vec::new()),
            }],
            superclass: object_type(),
            fields: Vec::new(),
            methods: Vec::new(),
        };
        let utility = GClass {
            name: "Utility".to_owned(),
            type_parameters: Vec::new(),
            superclass: object_type(),
            fields: Vec::new(),
            methods: vec![GMethod {
                type_parameters: vec![TypeParameter {
                    name: "Y".to_owned(),
                    bound: object_type(),
                }],
                result: GType::Variable("Y".to_owned()),
                name: "id".to_owned(),
                parameters: vec![(GType::Variable("Y".to_owned()), "value".to_owned())],
                body: GTerm::Variable("value".to_owned()),
            }],
        };
        let mut classes = vec![a, b, box_class, holder, utility];
        classes.extend(generic_auxiliary_classes());
        GenericTable::new(classes).expect("valid generic class table")
    }

    fn generic_auxiliary_classes() -> Vec<GClass> {
        let a_box = GClass {
            name: "ABox".to_owned(),
            type_parameters: Vec::new(),
            superclass: GType::Class(
                "Box".to_owned(),
                vec![GType::Class("A".to_owned(), Vec::new())],
            ),
            fields: Vec::new(),
            methods: Vec::new(),
        };
        let comparable = GClass {
            name: "Comparable".to_owned(),
            type_parameters: vec![TypeParameter {
                name: "X".to_owned(),
                bound: object_type(),
            }],
            superclass: object_type(),
            fields: Vec::new(),
            methods: vec![GMethod {
                type_parameters: Vec::new(),
                result: GType::Variable("X".to_owned()),
                name: "same".to_owned(),
                parameters: vec![(GType::Variable("X".to_owned()), "other".to_owned())],
                body: GTerm::Variable("other".to_owned()),
            }],
        };
        let score = GClass {
            name: "Score".to_owned(),
            type_parameters: Vec::new(),
            superclass: GType::Class(
                "Comparable".to_owned(),
                vec![GType::Class("Score".to_owned(), Vec::new())],
            ),
            fields: Vec::new(),
            methods: Vec::new(),
        };
        let ranked = GClass {
            name: "Ranked".to_owned(),
            type_parameters: vec![TypeParameter {
                name: "X".to_owned(),
                bound: GType::Class(
                    "Comparable".to_owned(),
                    vec![GType::Variable("X".to_owned())],
                ),
            }],
            superclass: object_type(),
            fields: Vec::new(),
            methods: Vec::new(),
        };
        let plain_pair = GClass {
            name: "PlainPair".to_owned(),
            type_parameters: Vec::new(),
            superclass: object_type(),
            fields: vec![
                (object_type(), "left".to_owned()),
                (object_type(), "right".to_owned()),
            ],
            methods: vec![GMethod {
                type_parameters: Vec::new(),
                result: object_type(),
                name: "first".to_owned(),
                parameters: Vec::new(),
                body: GTerm::Field(
                    Box::new(GTerm::Variable("this".to_owned())),
                    "left".to_owned(),
                ),
            }],
        };
        let user = GClass {
            name: "User".to_owned(),
            type_parameters: Vec::new(),
            superclass: object_type(),
            fields: Vec::new(),
            methods: vec![GMethod {
                type_parameters: Vec::new(),
                result: GType::Class("A".to_owned(), Vec::new()),
                name: "extract".to_owned(),
                parameters: vec![(
                    GType::Class(
                        "Box".to_owned(),
                        vec![GType::Class("A".to_owned(), Vec::new())],
                    ),
                    "box".to_owned(),
                )],
                body: GTerm::Invoke(
                    Box::new(GTerm::Variable("box".to_owned())),
                    "get".to_owned(),
                    Vec::new(),
                    Vec::new(),
                ),
            }],
        };
        vec![a_box, comparable, score, ranked, plain_pair, user]
    }

    #[test]
    fn pair_example_typechecks_and_evaluates() {
        let table = pair_table();
        let term = Term::Invoke(
            Box::new(Term::New(
                "Pair".to_owned(),
                vec![
                    Term::New("A".to_owned(), vec![]),
                    Term::New("B".to_owned(), vec![]),
                ],
            )),
            "setfst".to_owned(),
            vec![Term::New("B".to_owned(), vec![])],
        );
        assert_eq!(table.type_of(&HashMap::new(), &term).unwrap().class, "Pair");
        assert_eq!(
            table.eval(term, 20).unwrap(),
            Term::New(
                "Pair".to_owned(),
                vec![
                    Term::New("B".to_owned(), vec![]),
                    Term::New("B".to_owned(), vec![])
                ]
            )
        );
    }

    #[test]
    fn fields_methods_and_casts_follow_runtime_classes() {
        let table = pair_table();
        let up_then_down = Term::Cast(
            "A".to_owned(),
            Box::new(Term::Cast(
                "Object".to_owned(),
                Box::new(Term::New("A".to_owned(), vec![])),
            )),
        );
        assert_eq!(
            table.eval(up_then_down, 20),
            Ok(Term::New("A".to_owned(), vec![]))
        );
        let pair = Term::New(
            "Pair".to_owned(),
            vec![
                Term::New("A".to_owned(), vec![]),
                Term::New("B".to_owned(), vec![]),
            ],
        );
        let successful = Term::Field(
            Box::new(Term::Cast(
                "Pair".to_owned(),
                Box::new(Term::Cast(
                    "Object".to_owned(),
                    Box::new(Term::Field(Box::new(pair), "fst".to_owned())),
                )),
            )),
            "snd".to_owned(),
        );
        assert_eq!(
            table.eval(successful, 20),
            Err(FjError::FailedCast {
                runtime: "A".to_owned(),
                target: "Pair".to_owned(),
            })
        );
        let stupid = Term::Cast("A".to_owned(), Box::new(Term::New("B".to_owned(), vec![])));
        assert_eq!(
            table
                .type_of(&HashMap::new(), &stupid)
                .unwrap()
                .warnings
                .len(),
            1
        );
    }

    #[test]
    fn class_table_rejects_bad_constructors_overrides_and_cycles() {
        let mut bad_pair = pair_table().class("Pair").unwrap().clone();
        bad_pair.constructor.assignments.clear();
        assert!(matches!(
            ClassTable::new(vec![bad_pair]),
            Err(FjError::BadConstructor(_) | FjError::UnknownClass(_))
        ));

        let cycle_a = Class {
            name: "CycleA".to_owned(),
            superclass: "CycleB".to_owned(),
            fields: Vec::new(),
            constructor: empty_constructor(),
            methods: Vec::new(),
        };
        let cycle_b = Class {
            name: "CycleB".to_owned(),
            superclass: "CycleA".to_owned(),
            fields: Vec::new(),
            constructor: empty_constructor(),
            methods: Vec::new(),
        };
        assert!(matches!(
            ClassTable::new(vec![cycle_a, cycle_b]),
            Err(FjError::InheritanceCycle(_))
        ));

        let table = pair_table();
        let mut classes: Vec<_> = table.classes.values().cloned().collect();
        classes.push(Class {
            name: "BadPair".to_owned(),
            superclass: "Pair".to_owned(),
            fields: Vec::new(),
            constructor: Constructor {
                parameters: vec![
                    ("Object".to_owned(), "fst".to_owned()),
                    ("Object".to_owned(), "snd".to_owned()),
                ],
                super_arguments: vec!["fst".to_owned(), "snd".to_owned()],
                assignments: Vec::new(),
            },
            methods: vec![Method {
                result: "Object".to_owned(),
                name: "setfst".to_owned(),
                parameters: vec![("Object".to_owned(), "newfst".to_owned())],
                body: Term::Variable("newfst".to_owned()),
            }],
        });
        assert!(matches!(
            ClassTable::new(classes),
            Err(FjError::BadOverride(method)) if method == "setfst"
        ));
    }

    #[test]
    fn class_table_rejects_unknown_types_and_bad_parameter_names() {
        let bad_field = Class {
            name: "BadField".to_owned(),
            superclass: "Object".to_owned(),
            fields: vec![("Missing".to_owned(), "value".to_owned())],
            constructor: Constructor {
                parameters: vec![("Missing".to_owned(), "value".to_owned())],
                super_arguments: Vec::new(),
                assignments: vec!["value".to_owned()],
            },
            methods: Vec::new(),
        };
        assert!(matches!(
            ClassTable::new(vec![bad_field]),
            Err(FjError::UnknownClass(name)) if name == "Missing"
        ));

        let bad_parameters = Class {
            name: "BadParameters".to_owned(),
            superclass: "Object".to_owned(),
            fields: Vec::new(),
            constructor: empty_constructor(),
            methods: vec![Method {
                result: "Object".to_owned(),
                name: "choose".to_owned(),
                parameters: vec![
                    ("Object".to_owned(), "x".to_owned()),
                    ("Object".to_owned(), "x".to_owned()),
                ],
                body: Term::Variable("x".to_owned()),
            }],
        };
        assert!(matches!(
            ClassTable::new(vec![bad_parameters]),
            Err(FjError::DuplicateParameter(method)) if method == "choose"
        ));

        let reserved_parameter = Class {
            name: "ReservedParameter".to_owned(),
            superclass: "Object".to_owned(),
            fields: Vec::new(),
            constructor: empty_constructor(),
            methods: vec![Method {
                result: "Object".to_owned(),
                name: "bad".to_owned(),
                parameters: vec![("Object".to_owned(), "this".to_owned())],
                body: Term::Variable("this".to_owned()),
            }],
        };
        assert!(matches!(
            ClassTable::new(vec![reserved_parameter]),
            Err(FjError::ReservedParameter(method)) if method == "bad"
        ));

        let reserved_object = Class {
            name: "Object".to_owned(),
            superclass: "Object".to_owned(),
            fields: Vec::new(),
            constructor: empty_constructor(),
            methods: Vec::new(),
        };
        assert!(matches!(
            ClassTable::new(vec![reserved_object]),
            Err(FjError::ReservedClass(name)) if name == "Object"
        ));
    }

    #[test]
    fn typechecker_rejects_wrong_method_arity() {
        let table = pair_table();
        let call = Term::Invoke(
            Box::new(Term::New(
                "Pair".to_owned(),
                vec![
                    Term::New("A".to_owned(), Vec::new()),
                    Term::New("B".to_owned(), Vec::new()),
                ],
            )),
            "setfst".to_owned(),
            Vec::new(),
        );
        assert_eq!(
            table.type_of(&HashMap::new(), &call),
            Err(FjError::WrongArity {
                expected: 1,
                actual: 0,
            })
        );
    }

    #[test]
    fn location_cast_checks_the_stored_runtime_class() {
        let table = pair_table();
        let store = HashMap::from([(0, Term::New("A".to_owned(), Vec::new()))]);
        assert_eq!(eval_location_cast(&table, &store, "Object", 0), Ok(0));
        assert_eq!(
            eval_location_cast(&table, &store, "Pair", 0),
            Err(FjError::FailedCast {
                runtime: "A".to_owned(),
                target: "Pair".to_owned(),
            })
        );
    }

    #[test]
    fn generic_classes_methods_bounds_and_erasure_work() {
        let table = generic_table();
        let box_a = GType::Class(
            "Box".to_owned(),
            vec![GType::Class("A".to_owned(), Vec::new())],
        );
        let term = GTerm::Invoke(
            Box::new(GTerm::New(
                box_a,
                vec![GTerm::New(
                    GType::Class("A".to_owned(), Vec::new()),
                    Vec::new(),
                )],
            )),
            "get".to_owned(),
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(
            table.type_of(&HashMap::new(), &HashMap::new(), &term),
            Ok(GType::Class("A".to_owned(), Vec::new()))
        );
        let invalid = GType::Class(
            "Holder".to_owned(),
            vec![GType::Class("B".to_owned(), Vec::new())],
        );
        assert!(matches!(
            table.check_type(&invalid, &HashMap::new()),
            Err(GjError::BoundViolation { .. })
        ));
        let generic_identity = GTerm::Invoke(
            Box::new(GTerm::New(
                GType::Class("Utility".to_owned(), Vec::new()),
                Vec::new(),
            )),
            "id".to_owned(),
            vec![GType::Class("A".to_owned(), Vec::new())],
            vec![GTerm::New(
                GType::Class("A".to_owned(), Vec::new()),
                Vec::new(),
            )],
        );
        assert_eq!(
            table.type_of(&HashMap::new(), &HashMap::new(), &generic_identity),
            Ok(GType::Class("A".to_owned(), Vec::new()))
        );
        assert_eq!(
            table.eval_erased(&HashMap::new(), &HashMap::new(), &term, 20),
            Ok(Term::New("A".to_owned(), Vec::new()))
        );
        assert_eq!(
            table.eval_erased(&HashMap::new(), &HashMap::new(), &generic_identity, 20),
            Ok(Term::New("A".to_owned(), Vec::new()))
        );
    }

    #[test]
    fn generic_inheritance_recursive_bounds_and_bound_lookup_work() {
        let table = generic_table();
        let inherited = GTerm::Invoke(
            Box::new(GTerm::New(
                GType::Class("ABox".to_owned(), Vec::new()),
                vec![GTerm::New(
                    GType::Class("A".to_owned(), Vec::new()),
                    Vec::new(),
                )],
            )),
            "get".to_owned(),
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(
            table.type_of(&HashMap::new(), &HashMap::new(), &inherited),
            Ok(GType::Class("A".to_owned(), Vec::new()))
        );
        assert_eq!(
            table.eval_erased(&HashMap::new(), &HashMap::new(), &inherited, 20),
            Ok(Term::New("A".to_owned(), Vec::new()))
        );

        let recursive_bound = GType::Class(
            "Ranked".to_owned(),
            vec![GType::Class("Score".to_owned(), Vec::new())],
        );
        assert_eq!(table.check_type(&recursive_bound, &HashMap::new()), Ok(()));

        let variable = GType::Variable("X".to_owned());
        let bounds = HashMap::from([(
            "X".to_owned(),
            GType::Class("Comparable".to_owned(), vec![variable.clone()]),
        )]);
        assert_eq!(
            table.method("same", &variable, &bounds).unwrap().result,
            variable
        );
        let variable_field = GTerm::Field(
            Box::new(GTerm::Variable("box".to_owned())),
            "value".to_owned(),
        );
        let field_bounds = HashMap::from([(
            "X".to_owned(),
            GType::Class(
                "Box".to_owned(),
                vec![GType::Class("A".to_owned(), Vec::new())],
            ),
        )]);
        let field_context = HashMap::from([("box".to_owned(), GType::Variable("X".to_owned()))]);
        assert_eq!(
            table.type_of(&field_bounds, &field_context, &variable_field),
            Ok(GType::Class("A".to_owned(), Vec::new()))
        );
    }

    #[test]
    fn erased_nongeneric_program_preserves_its_result() {
        let table = generic_table();
        let nongeneric = GTerm::Invoke(
            Box::new(GTerm::New(
                GType::Class("PlainPair".to_owned(), Vec::new()),
                vec![
                    GTerm::New(GType::Class("A".to_owned(), Vec::new()), Vec::new()),
                    GTerm::New(GType::Class("B".to_owned(), Vec::new()), Vec::new()),
                ],
            )),
            "first".to_owned(),
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(
            table.eval_erased(&HashMap::new(), &HashMap::new(), &nongeneric, 20),
            Ok(Term::New("A".to_owned(), Vec::new()))
        );
    }

    #[test]
    fn type_directed_erasure_inserts_result_casts() {
        let table = generic_table();
        let box_a = GTerm::New(
            GType::Class(
                "Box".to_owned(),
                vec![GType::Class("A".to_owned(), Vec::new())],
            ),
            vec![GTerm::New(
                GType::Class("A".to_owned(), Vec::new()),
                Vec::new(),
            )],
        );
        let extract = GTerm::Invoke(
            Box::new(GTerm::New(
                GType::Class("User".to_owned(), Vec::new()),
                Vec::new(),
            )),
            "extract".to_owned(),
            Vec::new(),
            vec![box_a.clone()],
        );
        assert_eq!(
            table.eval_erased(&HashMap::new(), &HashMap::new(), &extract, 30),
            Ok(Term::New("A".to_owned(), Vec::new()))
        );

        let chained = GTerm::Invoke(
            Box::new(GTerm::Invoke(
                Box::new(box_a),
                "get".to_owned(),
                Vec::new(),
                Vec::new(),
            )),
            "self".to_owned(),
            Vec::new(),
            Vec::new(),
        );
        assert_eq!(
            table.eval_erased(&HashMap::new(), &HashMap::new(), &chained, 30),
            Ok(Term::New("A".to_owned(), Vec::new()))
        );
    }

    #[test]
    fn generic_table_rejects_reserved_object_and_changed_override_bounds() {
        let reserved_object = GClass {
            name: "Object".to_owned(),
            type_parameters: Vec::new(),
            superclass: object_type(),
            fields: Vec::new(),
            methods: Vec::new(),
        };
        assert!(matches!(
            GenericTable::new(vec![reserved_object]),
            Err(GjError::ReservedClass(name)) if name == "Object"
        ));

        let table = generic_table();
        let mut classes: Vec<_> = table.classes.values().cloned().collect();
        let generic_method = |bound: &str| GMethod {
            type_parameters: vec![TypeParameter {
                name: "Y".to_owned(),
                bound: GType::Class(bound.to_owned(), Vec::new()),
            }],
            result: GType::Variable("Y".to_owned()),
            name: "id".to_owned(),
            parameters: vec![(GType::Variable("Y".to_owned()), "value".to_owned())],
            body: GTerm::Variable("value".to_owned()),
        };
        classes.push(GClass {
            name: "Base".to_owned(),
            type_parameters: Vec::new(),
            superclass: object_type(),
            fields: Vec::new(),
            methods: vec![generic_method("A")],
        });
        classes.push(GClass {
            name: "Derived".to_owned(),
            type_parameters: Vec::new(),
            superclass: GType::Class("Base".to_owned(), Vec::new()),
            fields: Vec::new(),
            methods: vec![generic_method("B")],
        });
        assert!(matches!(
            GenericTable::new(classes),
            Err(GjError::BadOverride(name)) if name == "id"
        ));
    }

    #[test]
    fn generic_table_rejects_an_ill_typed_method_body() {
        let bad = GClass {
            name: "Bad".to_owned(),
            type_parameters: Vec::new(),
            superclass: object_type(),
            fields: Vec::new(),
            methods: vec![GMethod {
                type_parameters: Vec::new(),
                result: GType::Class("A".to_owned(), Vec::new()),
                name: "bad".to_owned(),
                parameters: Vec::new(),
                body: GTerm::New(GType::Class("B".to_owned(), Vec::new()), Vec::new()),
            }],
        };
        let a = GClass {
            name: "A".to_owned(),
            type_parameters: Vec::new(),
            superclass: object_type(),
            fields: Vec::new(),
            methods: Vec::new(),
        };
        let b = GClass {
            name: "B".to_owned(),
            type_parameters: Vec::new(),
            superclass: object_type(),
            fields: Vec::new(),
            methods: Vec::new(),
        };
        assert!(matches!(
            GenericTable::new(vec![a, b, bad]),
            Err(GjError::TypeMismatch { .. })
        ));
    }
}
