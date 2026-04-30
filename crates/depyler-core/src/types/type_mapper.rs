use crate::hir::{ConstGeneric, Type as PythonType};
use crate::annotations::{
    OwnershipModel, StringStrategy as AnnotationStringStrategy, TranspilationAnnotations,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IntWidth {
    I32,
    I64,
    ISize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StringStrategy {
    AlwaysOwned,    // String everywhere (safe, simple)
    InferBorrowing, // &str where possible (V1.1)
    CowByDefault,   // Cow<'static, str> (V1.2)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeMapper {
    pub width_preference: IntWidth,
    pub string_type: StringStrategy,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum RustType {
    Primitive(PrimitiveType),
    String,
    Str {
        lifetime: Option<String>,
    },
    Cow {
        lifetime: String,
    },
    Vec(Box<RustType>),
    HashMap(Box<RustType>, Box<RustType>),
    HashSet(Box<RustType>),
    Option(Box<RustType>),
    Result(Box<RustType>, Box<RustType>),
    Reference {
        lifetime: Option<String>,
        mutable: bool,
        inner: Box<RustType>,
    },
    Tuple(Vec<RustType>),
    Unit,
    Custom(String),
    Unsupported(String),
    /// Type parameter for generics
    TypeParam(String),
    /// Generic type with parameters
    Generic {
        base: String,
        params: Vec<RustType>,
    },
    /// Enum type for union types
    Enum {
        name: String,
        variants: Vec<(String, RustType)>,
    },
    /// Fixed-size array type
    Array {
        element_type: Box<RustType>,
        size: RustConstGeneric,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RustConstGeneric {
    /// Literal constant value (e.g., 5 in [T; 5])
    Literal(usize),
    /// Const generic parameter (e.g., N in [T; N])
    Parameter(String),
    /// Expression involving const generics (e.g., N + 1)
    Expression(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PrimitiveType {
    Bool,
    I8,
    I16,
    I32,
    I64,
    I128,
    ISize,
    U8,
    U16,
    U32,
    U64,
    U128,
    USize,
    F32,
    F64,
}

impl Default for TypeMapper {
    fn default() -> Self {
        Self {
            width_preference: IntWidth::I32,
            string_type: StringStrategy::AlwaysOwned,
        }
    }
}

impl TypeMapper {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_i64(mut self) -> Self {
        self.width_preference = IntWidth::I64;
        self
    }

    pub fn with_string_strategy(mut self, strategy: StringStrategy) -> Self {
        self.string_type = strategy;
        self
    }

    pub fn map_type(&self, py_type: &PythonType) -> RustType {
        match py_type {
            // This matches the pattern used for untyped Dict/List (lines 158-161)
            PythonType::Unknown => RustType::Custom("serde_json::Value".to_string()),
            PythonType::Int => RustType::Primitive(match self.width_preference {
                IntWidth::I32 => PrimitiveType::I32,
                IntWidth::I64 => PrimitiveType::I64,
                IntWidth::ISize => PrimitiveType::ISize,
            }),
            PythonType::Float => RustType::Primitive(PrimitiveType::F64),
            PythonType::String => match self.string_type {
                StringStrategy::AlwaysOwned => RustType::String,
                StringStrategy::InferBorrowing => RustType::String, // Always owned
                StringStrategy::CowByDefault => RustType::String,   // Always owned
            },
            PythonType::Bool => RustType::Primitive(PrimitiveType::Bool),
            PythonType::None => RustType::Unit,
            PythonType::List(inner) => RustType::Vec(Box::new(self.map_type(inner))),
            PythonType::Dict(k, v) => {
                RustType::HashMap(Box::new(self.map_type(k)), Box::new(self.map_type(v)))
            }
            PythonType::Tuple(types) => {
                let rust_types = types.iter().map(|t| self.map_type(t)).collect();
                RustType::Tuple(rust_types)
            }
            PythonType::Optional(inner) => RustType::Option(Box::new(self.map_type(inner))),
            PythonType::Final(inner) => self.map_type(inner), // Unwrap Final to get the actual type
            PythonType::Function { params: _, ret: _ } => {
                // For V1, we don't map function types directly
                RustType::Unsupported("function".to_string())
            }
            PythonType::Custom(name) => {
                // Check if this is a single uppercase letter (type parameter)
                if name.len() == 1 && name.chars().next().unwrap().is_uppercase() {
                    RustType::TypeParam(name.clone())
                } else {
                    // Handle common typing imports when used without parameters
                    match name.as_str() {
                        "Dict" => RustType::HashMap(
                            Box::new(RustType::String), // Default to String keys
                            Box::new(RustType::Custom("serde_json::Value".to_string())), // Default to JSON Value
                        ),
                        "List" => RustType::Vec(Box::new(RustType::Custom(
                            "serde_json::Value".to_string(),
                        ))),
                        "Set" => RustType::HashSet(Box::new(RustType::String)),
                        // Python `-> tuple` (without type parameters) maps to empty Rust tuple `()`
                        // This is a fallback - ideally type should be inferred from return value
                        "tuple" => RustType::Tuple(vec![]),
                        _ => RustType::Custom(name.clone()),
                    }
                }
            }
            PythonType::TypeVar(name) => RustType::TypeParam(name.clone()),
            PythonType::Generic { base, params } => {
                // Map generic types like MyClass<T> to appropriate Rust types
                match base.as_str() {
                    "List" if params.len() == 1 => {
                        RustType::Vec(Box::new(self.map_type(&params[0])))
                    }
                    "Dict" if params.len() == 2 => RustType::HashMap(
                        Box::new(self.map_type(&params[0])),
                        Box::new(self.map_type(&params[1])),
                    ),
                    _ => RustType::Generic {
                        base: base.clone(),
                        params: params.iter().map(|t| self.map_type(t)).collect(),
                    },
                }
            }
            PythonType::Union(types) => {
                // For now, map Union to an enum or use dynamic typing
                if types.len() == 2 && types.iter().any(|t| matches!(t, PythonType::None)) {
                    // Union[T, None] is Optional[T]
                    let non_none = types
                        .iter()
                        .find(|t| !matches!(t, PythonType::None))
                        .unwrap();
                    RustType::Option(Box::new(self.map_type(non_none)))
                } else {
                    // For non-optional unions, we'll need to generate an enum
                    // The actual enum will be generated during code generation
                    RustType::Enum {
                        name: "UnionType".to_string(), // Placeholder, will be replaced
                        variants: types
                            .iter()
                            .enumerate()
                            .map(|(i, t)| {
                                let variant_name = match t {
                                    PythonType::Int => "Integer".to_string(),
                                    PythonType::Float => "Float".to_string(),
                                    PythonType::String => "Text".to_string(),
                                    PythonType::Bool => "Boolean".to_string(),
                                    PythonType::None => "None".to_string(),
                                    _ => format!("Variant{}", i),
                                };
                                (variant_name, self.map_type(t))
                            })
                            .collect(),
                    }
                }
            }
            PythonType::Array { element_type, size } => RustType::Array {
                element_type: Box::new(self.map_type(element_type)),
                size: self.map_const_generic(size),
            },
            PythonType::Set(inner) => RustType::HashSet(Box::new(self.map_type(inner))),
        }
    }

    pub fn map_return_type(&self, py_type: &PythonType) -> RustType {
        match py_type {
            PythonType::None => RustType::Unit,
            PythonType::Unknown => RustType::Unit, // Functions without return annotation implicitly return None/()
            _ => self.map_type(py_type),
        }
    }

    pub fn needs_reference(&self, rust_type: &RustType) -> bool {
        match rust_type {
            RustType::String => false, // Always owned
            RustType::Vec(_) | RustType::HashMap(_, _) | RustType::HashSet(_) => true,
            RustType::Primitive(_) => false,
            RustType::Array { .. } => true, // Arrays need references for large sizes
            _ => false,
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    pub fn can_copy(&self, rust_type: &RustType) -> bool {
        match rust_type {
            RustType::Primitive(_) | RustType::Unit => true,
            RustType::Tuple(types) => types.iter().all(|t| self.can_copy(t)),
            RustType::Array { element_type, size } => {
                // Arrays are copy if elements are copy and size is reasonable
                match size {
                    RustConstGeneric::Literal(n) if *n <= 32 => self.can_copy(element_type),
                    _ => false, // Large or unknown size arrays are not Copy
                }
            }
            _ => false,
        }
    }

    /// Map a const generic from HIR to Rust representation
    pub fn map_const_generic(&self, const_generic: &ConstGeneric) -> RustConstGeneric {
        match const_generic {
            ConstGeneric::Literal(value) => RustConstGeneric::Literal(*value),
            ConstGeneric::Parameter(name) => RustConstGeneric::Parameter(name.clone()),
            ConstGeneric::Expression(expr) => RustConstGeneric::Expression(expr.clone()),
        }
    }
}

impl RustType {
    pub fn to_rust_string(&self) -> String {
        match self {
            RustType::Primitive(p) => p.to_rust_string().to_string(),
            RustType::String => "String".to_string(),
            RustType::Str { lifetime } => {
                if let Some(lt) = lifetime {
                    format!("&{lt} str")
                } else {
                    "&str".to_string()
                }
            }
            RustType::Cow { lifetime } => format!("Cow<{lifetime}, str>"),
            RustType::Vec(inner) => format!("Vec<{}>", inner.to_rust_string()),
            RustType::HashMap(k, v) => {
                format!("HashMap<{}, {}>", k.to_rust_string(), v.to_rust_string())
            }
            RustType::HashSet(inner) => format!("HashSet<{}>", inner.to_rust_string()),
            RustType::Option(inner) => format!("Option<{}>", inner.to_rust_string()),
            RustType::Result(ok, err) => {
                format!("Result<{}, {}>", ok.to_rust_string(), err.to_rust_string())
            }
            RustType::Reference {
                lifetime,
                mutable,
                inner,
            } => {
                let mut_str = if *mutable { "mut " } else { "" };
                if let Some(lt) = lifetime {
                    format!("&{} {}{}", lt, mut_str, inner.to_rust_string())
                } else {
                    format!("&{}{}", mut_str, inner.to_rust_string())
                }
            }
            RustType::Tuple(types) => {
                if types.is_empty() {
                    "()".to_string()
                } else {
                    let type_strs: Vec<String> = types.iter().map(|t| t.to_rust_string()).collect();
                    format!("({})", type_strs.join(", "))
                }
            }
            RustType::Unit => "()".to_string(),
            RustType::Custom(name) => name.clone(),
            RustType::Unsupported(desc) => format!("/* unsupported: {desc} */"),
            RustType::TypeParam(name) => name.clone(),
            RustType::Generic { base, params } => {
                let param_strs: Vec<String> = params.iter().map(|p| p.to_rust_string()).collect();
                format!("{}<{}>", base, param_strs.join(", "))
            }
            RustType::Enum { name, .. } => name.clone(),
            RustType::Array { element_type, size } => {
                format!(
                    "[{}; {}]",
                    element_type.to_rust_string(),
                    size.to_rust_string()
                )
            }
        }
    }
}

impl RustConstGeneric {
    pub fn to_rust_string(&self) -> String {
        match self {
            RustConstGeneric::Literal(value) => value.to_string(),
            RustConstGeneric::Parameter(name) => name.clone(),
            RustConstGeneric::Expression(expr) => expr.clone(),
        }
    }
}

impl PrimitiveType {
    pub fn to_rust_string(&self) -> &'static str {
        match self {
            PrimitiveType::Bool => "bool",
            PrimitiveType::I8 => "i8",
            PrimitiveType::I16 => "i16",
            PrimitiveType::I32 => "i32",
            PrimitiveType::I64 => "i64",
            PrimitiveType::I128 => "i128",
            PrimitiveType::ISize => "isize",
            PrimitiveType::U8 => "u8",
            PrimitiveType::U16 => "u16",
            PrimitiveType::U32 => "u32",
            PrimitiveType::U64 => "u64",
            PrimitiveType::U128 => "u128",
            PrimitiveType::USize => "usize",
            PrimitiveType::F32 => "f32",
            PrimitiveType::F64 => "f64",
        }
    }
}

// ---------------------------------------------------------------------------
// Annotation-aware type mapper (merged from annotation_aware_type_mapper.rs)
// ---------------------------------------------------------------------------

/// An enhanced type mapper that considers annotations when mapping types
pub struct AnnotationAwareTypeMapper {
    base_mapper: TypeMapper,
}

impl Default for AnnotationAwareTypeMapper {
    fn default() -> Self {
        Self::new()
    }
}

impl AnnotationAwareTypeMapper {
    pub fn new() -> Self {
        Self {
            base_mapper: TypeMapper::new(),
        }
    }

    pub fn with_base_mapper(base_mapper: TypeMapper) -> Self {
        Self { base_mapper }
    }

    /// Maps a Python type to a Rust type, considering the provided annotations
    pub fn map_type_with_annotations(
        &self,
        py_type: &PythonType,
        annotations: &TranspilationAnnotations,
    ) -> RustType {
        match py_type {
            PythonType::String => self.map_string_type(annotations),
            PythonType::List(inner) => self.map_list_type(inner, annotations),
            PythonType::Dict(key, value) => self.map_dict_type(key, value, annotations),
            PythonType::Optional(inner) => self.map_optional_type(inner, annotations),
            _ => self.base_mapper.map_type(py_type),
        }
    }

    /// Maps string types based on annotations
    fn map_string_type(&self, annotations: &TranspilationAnnotations) -> RustType {
        match annotations.string_strategy {
            AnnotationStringStrategy::AlwaysOwned => RustType::String,
            AnnotationStringStrategy::ZeroCopy => match annotations.ownership_model {
                OwnershipModel::Borrowed => RustType::Str {
                    lifetime: Some("'a".to_string()),
                },
                _ => RustType::String,
            },
            AnnotationStringStrategy::Conservative => match annotations.ownership_model {
                OwnershipModel::Borrowed => RustType::Str {
                    lifetime: Some("'a".to_string()),
                },
                _ => RustType::String,
            },
        }
    }

    /// Maps list types based on annotations
    fn map_list_type(
        &self,
        inner: &PythonType,
        annotations: &TranspilationAnnotations,
    ) -> RustType {
        let inner_rust = self.map_type_with_annotations(inner, annotations);

        match annotations.ownership_model {
            OwnershipModel::Borrowed => RustType::Reference {
                lifetime: Some("'a".to_string()),
                mutable: false,
                inner: Box::new(RustType::Vec(Box::new(inner_rust))),
            },
            OwnershipModel::Shared => {
                if annotations.thread_safety == crate::annotations::ThreadSafety::Required {
                    RustType::Custom(format!("Arc<Vec<{}>>", inner_rust.to_rust_string()))
                } else {
                    RustType::Custom(format!("Rc<Vec<{}>>", inner_rust.to_rust_string()))
                }
            }
            OwnershipModel::Owned => RustType::Vec(Box::new(inner_rust)),
        }
    }

    /// Maps dictionary types based on annotations
    fn map_dict_type(
        &self,
        key: &PythonType,
        value: &PythonType,
        annotations: &TranspilationAnnotations,
    ) -> RustType {
        let key_rust = self.map_type_with_annotations(key, annotations);
        let value_rust = self.map_type_with_annotations(value, annotations);

        // FnvHashMap and AHashMap require external crate dependencies that may not be available.
        // For standalone files, we prioritize compilation success over optimization.
        let hash_map_type = "HashMap";

        let base_type = RustType::Custom(format!(
            "{}<{}, {}>",
            hash_map_type,
            key_rust.to_rust_string(),
            value_rust.to_rust_string()
        ));

        match annotations.ownership_model {
            OwnershipModel::Borrowed => RustType::Reference {
                lifetime: Some("'a".to_string()),
                mutable: false,
                inner: Box::new(base_type),
            },
            OwnershipModel::Shared => {
                if annotations.thread_safety == crate::annotations::ThreadSafety::Required {
                    RustType::Custom(format!("Arc<{}>", base_type.to_rust_string()))
                } else {
                    RustType::Custom(format!("Rc<{}>", base_type.to_rust_string()))
                }
            }
            OwnershipModel::Owned => base_type,
        }
    }

    /// Maps optional types based on annotations
    fn map_optional_type(
        &self,
        inner: &PythonType,
        annotations: &TranspilationAnnotations,
    ) -> RustType {
        let inner_rust = self.map_type_with_annotations(inner, annotations);

        match annotations.error_strategy {
            crate::annotations::ErrorStrategy::ResultType => RustType::Result(
                Box::new(inner_rust),
                Box::new(RustType::Custom("Error".to_string())),
            ),
            _ => RustType::Option(Box::new(inner_rust)),
        }
    }

    /// Determines if a type should be passed by reference based on annotations
    pub fn needs_reference_with_annotations(
        &self,
        rust_type: &RustType,
        annotations: &TranspilationAnnotations,
    ) -> bool {
        match annotations.ownership_model {
            OwnershipModel::Borrowed => !self.base_mapper.can_copy(rust_type),
            OwnershipModel::Owned => false,
            OwnershipModel::Shared => false,
        }
    }

    /// Maps return types considering annotations
    pub fn map_return_type_with_annotations(
        &self,
        py_type: &PythonType,
        annotations: &TranspilationAnnotations,
    ) -> RustType {
        match py_type {
            PythonType::None => match annotations.error_strategy {
                crate::annotations::ErrorStrategy::ResultType => RustType::Result(
                    Box::new(RustType::Unit),
                    Box::new(RustType::Custom("Error".to_string())),
                ),
                _ => RustType::Unit,
            },
            PythonType::Unknown => RustType::Unit,
            _ => self.map_type_with_annotations(py_type, annotations),
        }
    }
}
