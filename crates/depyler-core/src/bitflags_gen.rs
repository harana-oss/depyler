//! Bitflags code generation for string set optimization.
//!
//! Generates Rust bitflags from detected string set patterns.

use crate::string_set_detection::{
    InvalidStringValue, StringSetCandidate, normalize_to_rust_identifier, validate_string_values,
};
use std::collections::HashMap;

/// Optimization decision for converting a string set to bitflags.
#[derive(Debug, Clone)]
pub struct BitflagsDecision {
    pub candidate: StringSetCandidate,
    pub target_type: BitflagsTargetType,
    pub flag_mapping: HashMap<String, u64>,
}

/// Target type for bitflags based on number of values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitflagsTargetType {
    U8,  // ≤8 values
    U16, // ≤16 values
    U32, // ≤32 values
    U64, // ≤64 values
}

impl BitflagsTargetType {
    pub fn from_count(count: usize) -> Option<Self> {
        match count {
            0..=8 => Some(Self::U8),
            9..=16 => Some(Self::U16),
            17..=32 => Some(Self::U32),
            33..=64 => Some(Self::U64),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
        }
    }
}

/// Bitflags code generator.
#[derive(Debug)]
pub struct BitflagsGenerator {
    decisions: Vec<BitflagsDecision>,
}

impl Default for BitflagsGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl BitflagsGenerator {
    pub fn new() -> Self {
        Self { decisions: vec![] }
    }

    /// Create a bitflags decision from a candidate.
    pub fn create_decision(&self, candidate: StringSetCandidate) -> Result<BitflagsDecision, BitflagsError> {
        let count = candidate.values.len();

        if count == 0 {
            return Err(BitflagsError::EmptySet);
        }

        let target_type = BitflagsTargetType::from_count(count).ok_or(BitflagsError::TooManyValues { count })?;

        // Create flag mapping
        let rust_idents = validate_string_values(&candidate.values).map_err(|e| BitflagsError::InvalidValue(e))?;

        let mut flag_mapping = HashMap::new();
        for (i, (orig, ident)) in candidate.values.iter().zip(rust_idents.iter()).enumerate() {
            flag_mapping.insert(orig.clone(), 1u64 << i);
            flag_mapping.insert(ident.clone(), 1u64 << i);
        }

        Ok(BitflagsDecision {
            candidate,
            target_type,
            flag_mapping,
        })
    }

    /// Add a decision.
    pub fn add_decision(&mut self, decision: BitflagsDecision) {
        self.decisions.push(decision);
    }

    /// Generate bitflags definitions.
    pub fn generate_definitions(&self) -> String {
        let mut output = String::new();

        for decision in &self.decisions {
            output.push_str(&self.generate_bitflags_struct(&decision));
            output.push('\n');
        }

        output
    }

    /// Generate a single bitflags struct.
    fn generate_bitflags_struct(&self, decision: &BitflagsDecision) -> String {
        let struct_name = to_pascal_case(&decision.candidate.name);
        let target_type = decision.target_type.as_str();

        let rust_idents = validate_string_values(&decision.candidate.values).unwrap_or_else(|_| {
            decision
                .candidate
                .values
                .iter()
                .map(|v| normalize_to_rust_identifier(v))
                .collect()
        });

        let mut flags = String::new();
        for (i, ident) in rust_idents.iter().enumerate() {
            let bit_value = 1u64 << i;
            flags.push_str(&format!("        const {} = {:#x};\n", ident, bit_value));
        }

        format!(
            r#"bitflags! {{
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct {}: {} {{
{}    }}
}}

impl {} {{
    pub fn from_str(s: &str) -> Option<Self> {{
        match s {{
{}            _ => None,
        }}
    }}

    pub fn as_str(&self) -> &'static str {{
        match *self {{
{}            _ => "",
        }}
    }}
}}
"#,
            struct_name,
            target_type,
            flags,
            struct_name,
            self.generate_from_str_arms(&decision.candidate.values, &rust_idents, &struct_name),
            self.generate_as_str_arms(&decision.candidate.values, &rust_idents, &struct_name),
        )
    }

    fn generate_from_str_arms(&self, values: &[String], idents: &[String], _struct_name: &str) -> String {
        let mut arms = String::new();
        for (value, ident) in values.iter().zip(idents.iter()) {
            arms.push_str(&format!("            \"{}\" => Some(Self::{}),\n", value, ident));
        }
        arms
    }

    fn generate_as_str_arms(&self, values: &[String], idents: &[String], _struct_name: &str) -> String {
        let mut arms = String::new();
        for (value, ident) in values.iter().zip(idents.iter()) {
            arms.push_str(&format!("            Self::{} => \"{}\",\n", ident, value));
        }
        arms
    }
}

/// Convert snake_case or SCREAMING_CASE to PascalCase.
fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
            }
        })
        .collect()
}

/// Errors during bitflags generation.
#[derive(Debug, Clone)]
pub enum BitflagsError {
    EmptySet,
    TooManyValues { count: usize },
    InvalidValue(InvalidStringValue),
}

impl std::fmt::Display for BitflagsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptySet => write!(f, "Cannot generate bitflags from empty set"),
            Self::TooManyValues { count } => {
                write!(f, "Set has {} values, exceeding maximum of 64", count)
            }
            Self::InvalidValue(e) => write!(f, "Invalid string value: {:?}", e),
        }
    }
}

impl std::error::Error for BitflagsError {}

/// Operation mapping from Python to Rust bitflags.
#[derive(Debug, Clone, Copy)]
pub enum BitflagsOperation {
    Contains,     // x in set -> flags.contains(Flag::X)
    Union,        // set1 | set2 -> flags1 | flags2
    Intersection, // set1 & set2 -> flags1 & flags2
    Difference,   // set1 - set2 -> flags1 & !flags2
    Length,       // len(set) -> flags.bits().count_ones()
    Insert,       // set.add(x) -> flags.insert(Flag::X)
    Remove,       // set.remove(x) -> flags.remove(Flag::X)
}

impl BitflagsOperation {
    pub fn generate_code(&self, flags_var: &str, operand: Option<&str>) -> String {
        match self {
            Self::Contains => {
                format!("{}.contains({})", flags_var, operand.unwrap_or("flag"))
            }
            Self::Union => {
                format!("{} | {}", flags_var, operand.unwrap_or("other"))
            }
            Self::Intersection => {
                format!("{} & {}", flags_var, operand.unwrap_or("other"))
            }
            Self::Difference => {
                format!("{} & !{}", flags_var, operand.unwrap_or("other"))
            }
            Self::Length => {
                format!("{}.bits().count_ones()", flags_var)
            }
            Self::Insert => {
                format!("{}.insert({})", flags_var, operand.unwrap_or("flag"))
            }
            Self::Remove => {
                format!("{}.remove({})", flags_var, operand.unwrap_or("flag"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::Span;
    use crate::string_set_detection::DetectionEvidence;

    #[test]
    fn test_bitflags_target_type_from_count() {
        assert_eq!(BitflagsTargetType::from_count(1), Some(BitflagsTargetType::U8));
        assert_eq!(BitflagsTargetType::from_count(8), Some(BitflagsTargetType::U8));
        assert_eq!(BitflagsTargetType::from_count(9), Some(BitflagsTargetType::U16));
        assert_eq!(BitflagsTargetType::from_count(16), Some(BitflagsTargetType::U16));
        assert_eq!(BitflagsTargetType::from_count(32), Some(BitflagsTargetType::U32));
        assert_eq!(BitflagsTargetType::from_count(64), Some(BitflagsTargetType::U64));
        assert_eq!(BitflagsTargetType::from_count(65), None);
    }

    #[test]
    fn test_to_pascal_case() {
        assert_eq!(to_pascal_case("PERMISSIONS"), "Permissions");
        assert_eq!(to_pascal_case("STATUS_FLAGS"), "StatusFlags");
        assert_eq!(to_pascal_case("user_roles"), "UserRoles");
    }

    #[test]
    fn test_create_decision() {
        let generator = BitflagsGenerator::new();
        let candidate = StringSetCandidate {
            name: "PERMISSIONS".to_string(),
            values: vec!["read".to_string(), "write".to_string(), "execute".to_string()],
            span: Span::default(),
            confidence: 0.9,
            evidence: vec![DetectionEvidence::AllLiterals],
        };

        let decision = generator.create_decision(candidate).unwrap();
        assert_eq!(decision.target_type, BitflagsTargetType::U8);
        assert_eq!(decision.flag_mapping.get("read"), Some(&1));
        assert_eq!(decision.flag_mapping.get("write"), Some(&2));
        assert_eq!(decision.flag_mapping.get("execute"), Some(&4));
    }

    #[test]
    fn test_create_decision_empty_set() {
        let generator = BitflagsGenerator::new();
        let candidate = StringSetCandidate {
            name: "EMPTY".to_string(),
            values: vec![],
            span: Span::default(),
            confidence: 0.0,
            evidence: vec![],
        };

        let result = generator.create_decision(candidate);
        assert!(matches!(result, Err(BitflagsError::EmptySet)));
    }

    #[test]
    fn test_generate_bitflags_struct() {
        let generator = BitflagsGenerator::new();
        let candidate = StringSetCandidate {
            name: "PERMISSIONS".to_string(),
            values: vec!["read".to_string(), "write".to_string()],
            span: Span::default(),
            confidence: 0.9,
            evidence: vec![],
        };

        let decision = generator.create_decision(candidate).unwrap();
        let output = generator.generate_bitflags_struct(&decision);

        assert!(output.contains("pub struct Permissions: u8"));
        assert!(output.contains("const READ = 0x1"));
        assert!(output.contains("const WRITE = 0x2"));
        assert!(output.contains("fn from_str(s: &str)"));
        assert!(output.contains("fn as_str(&self)"));
    }

    #[test]
    fn test_bitflags_operation_generate_code() {
        let op = BitflagsOperation::Contains;
        assert_eq!(
            op.generate_code("permissions", Some("Permissions::READ")),
            "permissions.contains(Permissions::READ)"
        );

        let op = BitflagsOperation::Union;
        assert_eq!(op.generate_code("flags1", Some("flags2")), "flags1 | flags2");

        let op = BitflagsOperation::Difference;
        assert_eq!(op.generate_code("flags1", Some("flags2")), "flags1 & !flags2");

        let op = BitflagsOperation::Length;
        assert_eq!(op.generate_code("flags", None), "flags.bits().count_ones()");
    }
}
