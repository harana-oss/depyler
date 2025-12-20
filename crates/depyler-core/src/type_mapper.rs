use crate::hir::{ConstGeneric, Type as PythonType};
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
                StringStrategy::InferBorrowing => RustType::String, // V1: Always owned
                StringStrategy::CowByDefault => RustType::String,   // V1: Always owned
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
            RustType::String => false, // V1: Always owned
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
