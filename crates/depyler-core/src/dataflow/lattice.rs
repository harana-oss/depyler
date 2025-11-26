//! Type lattice for dataflow analysis
//!
//! Implements a lattice structure over types where:
//! - Bottom (⊥) = Unreachable/undefined
//! - Top (⊤) = Unknown/any type
//! - Concrete types form the middle of the lattice

use crate::hir::Type;
use std::collections::HashMap;

/// A type in the lattice
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LatticeType {
    /// Bottom element - unreachable or uninitialized
    Bottom,
    /// A concrete type
    Concrete(Type),
    /// Top element - could be any type (conflicting assignments)
    Top,
}

impl LatticeType {
    /// Join operation (least upper bound)
    /// Combines two types to find their most specific common supertype
    pub fn join(&self, other: &LatticeType) -> LatticeType {
        match (self, other) {
            // Bottom is identity for join
            (LatticeType::Bottom, t) | (t, LatticeType::Bottom) => t.clone(),
            // Top absorbs everything
            (LatticeType::Top, _) | (_, LatticeType::Top) => LatticeType::Top,
            // Same concrete types stay the same
            (LatticeType::Concrete(t1), LatticeType::Concrete(t2)) => {
                if types_compatible(t1, t2) {
                    LatticeType::Concrete(join_types(t1, t2))
                } else {
                    // Incompatible types - go to top (or union if we support it)
                    LatticeType::Top
                }
            }
        }
    }

    /// Meet operation (greatest lower bound)
    /// Finds the most general type that is a subtype of both
    pub fn meet(&self, other: &LatticeType) -> LatticeType {
        match (self, other) {
            // Top is identity for meet
            (LatticeType::Top, t) | (t, LatticeType::Top) => t.clone(),
            // Bottom absorbs everything
            (LatticeType::Bottom, _) | (_, LatticeType::Bottom) => LatticeType::Bottom,
            // Concrete types
            (LatticeType::Concrete(t1), LatticeType::Concrete(t2)) => {
                if types_compatible(t1, t2) {
                    LatticeType::Concrete(meet_types(t1, t2))
                } else {
                    LatticeType::Bottom
                }
            }
        }
    }

    /// Check if this type is more specific (lower in lattice) than other
    pub fn is_subtype_of(&self, other: &LatticeType) -> bool {
        match (self, other) {
            (LatticeType::Bottom, _) => true,
            (_, LatticeType::Top) => true,
            (LatticeType::Concrete(t1), LatticeType::Concrete(t2)) => type_subtype(t1, t2),
            _ => false,
        }
    }

    /// Convert to HIR Type, defaulting to Unknown for Top/Bottom
    pub fn to_hir_type(&self) -> Type {
        match self {
            LatticeType::Concrete(t) => t.clone(),
            LatticeType::Bottom | LatticeType::Top => Type::Unknown,
        }
    }

    /// Create from HIR Type
    pub fn from_hir_type(ty: &Type) -> Self {
        if matches!(ty, Type::Unknown) {
            LatticeType::Top
        } else {
            LatticeType::Concrete(ty.clone())
        }
    }
}

/// Check if two types are compatible (can be unified)
fn types_compatible(t1: &Type, t2: &Type) -> bool {
    match (t1, t2) {
        (Type::Unknown, _) | (_, Type::Unknown) => true,
        (Type::Int, Type::Int) => true,
        (Type::Float, Type::Float) => true,
        (Type::Int, Type::Float) | (Type::Float, Type::Int) => true, // Numeric promotion
        (Type::String, Type::String) => true,
        (Type::Bool, Type::Bool) => true,
        (Type::None, Type::None) => true,
        (Type::List(e1), Type::List(e2)) => types_compatible(e1, e2),
        (Type::Dict(k1, v1), Type::Dict(k2, v2)) => {
            types_compatible(k1, k2) && types_compatible(v1, v2)
        }
        (Type::Tuple(ts1), Type::Tuple(ts2)) => {
            ts1.len() == ts2.len() && ts1.iter().zip(ts2).all(|(t1, t2)| types_compatible(t1, t2))
        }
        (Type::Set(e1), Type::Set(e2)) => types_compatible(e1, e2),
        (Type::Optional(inner1), Type::Optional(inner2)) => types_compatible(inner1, inner2),
        (Type::Optional(inner), other) => types_compatible(inner, other),
        (other, Type::Optional(inner)) => types_compatible(other, inner),
        // None is compatible with any type (forms Optional)
        (Type::None, _) | (_, Type::None) => true,
        (Type::Custom(n1), Type::Custom(n2)) => n1 == n2,
        (Type::Function { params: p1, ret: r1 }, Type::Function { params: p2, ret: r2 }) => {
            p1.len() == p2.len()
                && p1.iter().zip(p2).all(|(t1, t2)| types_compatible(t1, t2))
                && types_compatible(r1, r2)
        }
        _ => false,
    }
}

/// Join two compatible types (find their common supertype)
fn join_types(t1: &Type, t2: &Type) -> Type {
    match (t1, t2) {
        (Type::Unknown, t) | (t, Type::Unknown) => t.clone(),
        (Type::Int, Type::Float) | (Type::Float, Type::Int) => Type::Float,
        (Type::List(e1), Type::List(e2)) => Type::List(Box::new(join_types(e1, e2))),
        (Type::Dict(k1, v1), Type::Dict(k2, v2)) => {
            Type::Dict(Box::new(join_types(k1, k2)), Box::new(join_types(v1, v2)))
        }
        (Type::Tuple(ts1), Type::Tuple(ts2)) if ts1.len() == ts2.len() => {
            Type::Tuple(ts1.iter().zip(ts2).map(|(t1, t2)| join_types(t1, t2)).collect())
        }
        (Type::Set(e1), Type::Set(e2)) => Type::Set(Box::new(join_types(e1, e2))),
        (Type::Optional(inner1), Type::Optional(inner2)) => {
            Type::Optional(Box::new(join_types(inner1, inner2)))
        }
        (Type::Optional(inner), other) => {
            Type::Optional(Box::new(join_types(inner, other)))
        }
        (other, Type::Optional(inner)) => {
            Type::Optional(Box::new(join_types(other, inner)))
        }
        (Type::None, t) | (t, Type::None) => Type::Optional(Box::new(t.clone())),
        _ => t1.clone(), // Same type or incompatible
    }
}

/// Meet two types (find their common subtype)
fn meet_types(t1: &Type, t2: &Type) -> Type {
    match (t1, t2) {
        (Type::Unknown, t) | (t, Type::Unknown) => t.clone(),
        (Type::Int, Type::Float) | (Type::Float, Type::Int) => Type::Int,
        (Type::List(e1), Type::List(e2)) => Type::List(Box::new(meet_types(e1, e2))),
        (Type::Optional(inner1), Type::Optional(inner2)) => {
            Type::Optional(Box::new(meet_types(inner1, inner2)))
        }
        (Type::Optional(inner), other) => meet_types(inner, other),
        (other, Type::Optional(inner)) => meet_types(other, inner),
        _ => t1.clone(),
    }
}

/// Check if t1 is a subtype of t2
fn type_subtype(t1: &Type, t2: &Type) -> bool {
    match (t1, t2) {
        (_, Type::Unknown) => true,
        (Type::Int, Type::Float) => true, // Numeric widening
        (t1, Type::Optional(t2)) => type_subtype(t1, t2) || matches!(t1, Type::None),
        (Type::List(e1), Type::List(e2)) => type_subtype(e1, e2),
        (Type::Dict(k1, v1), Type::Dict(k2, v2)) => type_subtype(k1, k2) && type_subtype(v1, v2),
        (t1, t2) => t1 == t2,
    }
}

/// Type state for a single program point (maps variables to lattice types)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeState {
    pub vars: HashMap<String, LatticeType>,
}

impl TypeState {
    pub fn new() -> Self {
        Self {
            vars: HashMap::new(),
        }
    }

    /// Create a bottom state (all variables undefined)
    pub fn bottom() -> Self {
        Self::new()
    }

    /// Create a top state for given variables
    pub fn top_for_vars(vars: &[String]) -> Self {
        let mut state = Self::new();
        for var in vars {
            state.vars.insert(var.clone(), LatticeType::Top);
        }
        state
    }

    /// Get the type of a variable
    pub fn get(&self, var: &str) -> LatticeType {
        self.vars.get(var).cloned().unwrap_or(LatticeType::Bottom)
    }

    /// Set the type of a variable
    pub fn set(&mut self, var: String, ty: LatticeType) {
        self.vars.insert(var, ty);
    }

    /// Join two states (combine at merge points)
    pub fn join(&self, other: &TypeState) -> TypeState {
        let mut result = TypeState::new();
        
        // Join all variables from both states
        let all_vars: std::collections::HashSet<_> = 
            self.vars.keys().chain(other.vars.keys()).collect();
        
        for var in all_vars {
            let t1 = self.get(var);
            let t2 = other.get(var);
            result.set(var.clone(), t1.join(&t2));
        }
        
        result
    }

    /// Meet two states
    pub fn meet(&self, other: &TypeState) -> TypeState {
        let mut result = TypeState::new();
        
        // Only include variables present in both states
        for (var, t1) in &self.vars {
            if let Some(t2) = other.vars.get(var) {
                result.set(var.clone(), t1.meet(t2));
            }
        }
        
        result
    }

    /// Check if this state is equal to another (for fixpoint detection)
    pub fn equals(&self, other: &TypeState) -> bool {
        self == other
    }

    /// Check if this state is a subset (more specific) than other
    pub fn is_subset_of(&self, other: &TypeState) -> bool {
        for (var, ty) in &self.vars {
            let other_ty = other.get(var);
            if !ty.is_subtype_of(&other_ty) {
                return false;
            }
        }
        true
    }
}

impl Default for TypeState {
    fn default() -> Self {
        Self::new()
    }
}

/// The type lattice structure
pub struct TypeLattice;

impl TypeLattice {
    /// Infer type from a literal expression
    pub fn type_of_literal(lit: &crate::hir::Literal) -> Type {
        match lit {
            crate::hir::Literal::Int(_) => Type::Int,
            crate::hir::Literal::Float(_) => Type::Float,
            crate::hir::Literal::String(_) => Type::String,
            crate::hir::Literal::Bytes(_) => Type::Custom("bytes".to_string()),
            crate::hir::Literal::Bool(_) => Type::Bool,
            crate::hir::Literal::None => Type::None,
        }
    }

    /// Infer the result type of a binary operation
    pub fn binary_op_type(op: crate::hir::BinOp, left: &Type, right: &Type) -> Type {
        use crate::hir::BinOp;
        
        match op {
            // Arithmetic - result type depends on operands
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Mod => {
                match (left, right) {
                    (Type::Float, _) | (_, Type::Float) => Type::Float,
                    (Type::Int, Type::Int) => Type::Int,
                    (Type::String, Type::String) if matches!(op, BinOp::Add) => Type::String,
                    (Type::List(e), Type::List(_)) if matches!(op, BinOp::Add) => Type::List(e.clone()),
                    _ => Type::Unknown,
                }
            }
            BinOp::Div => Type::Float, // Python 3 true division
            BinOp::FloorDiv => Type::Int,
            BinOp::Pow => {
                match (left, right) {
                    (Type::Float, _) | (_, Type::Float) => Type::Float,
                    (Type::Int, Type::Int) => Type::Int,
                    _ => Type::Unknown,
                }
            }
            // Comparison - always bool
            BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::LtEq | BinOp::Gt | BinOp::GtEq => Type::Bool,
            // Logical - always bool
            BinOp::And | BinOp::Or => Type::Bool,
            // Bitwise - int only
            BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor | BinOp::LShift | BinOp::RShift => Type::Int,
            // Membership - always bool
            BinOp::In | BinOp::NotIn => Type::Bool,
        }
    }

    /// Infer the result type of a unary operation
    pub fn unary_op_type(op: crate::hir::UnaryOp, operand: &Type) -> Type {
        use crate::hir::UnaryOp;
        
        match op {
            UnaryOp::Not => Type::Bool,
            UnaryOp::Neg | UnaryOp::Pos => operand.clone(),
            UnaryOp::BitNot => Type::Int,
        }
    }

    /// Get element type of a container
    pub fn element_type(container: &Type) -> Type {
        match container {
            Type::List(elem) => *elem.clone(),
            Type::Set(elem) => *elem.clone(),
            Type::Tuple(elems) if !elems.is_empty() => elems[0].clone(),
            Type::Dict(_, val) => *val.clone(),
            Type::String => Type::String,
            Type::Array { element_type, .. } => *element_type.clone(),
            _ => Type::Unknown,
        }
    }

    /// Get key type of a dict
    pub fn key_type(container: &Type) -> Type {
        match container {
            Type::Dict(key, _) => *key.clone(),
            _ => Type::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lattice_join_bottom() {
        let bottom = LatticeType::Bottom;
        let int_type = LatticeType::Concrete(Type::Int);
        
        assert_eq!(bottom.join(&int_type), int_type);
        assert_eq!(int_type.join(&bottom), int_type);
    }

    #[test]
    fn test_lattice_join_top() {
        let top = LatticeType::Top;
        let int_type = LatticeType::Concrete(Type::Int);
        
        assert_eq!(top.join(&int_type), LatticeType::Top);
        assert_eq!(int_type.join(&top), LatticeType::Top);
    }

    #[test]
    fn test_lattice_join_same_type() {
        let int1 = LatticeType::Concrete(Type::Int);
        let int2 = LatticeType::Concrete(Type::Int);
        
        assert_eq!(int1.join(&int2), LatticeType::Concrete(Type::Int));
    }

    #[test]
    fn test_lattice_join_numeric_promotion() {
        let int_type = LatticeType::Concrete(Type::Int);
        let float_type = LatticeType::Concrete(Type::Float);
        
        // Int and Float join to Float
        let result = int_type.join(&float_type);
        assert_eq!(result, LatticeType::Concrete(Type::Float));
    }

    #[test]
    fn test_lattice_join_incompatible() {
        let int_type = LatticeType::Concrete(Type::Int);
        let str_type = LatticeType::Concrete(Type::String);
        
        // Incompatible types go to Top
        assert_eq!(int_type.join(&str_type), LatticeType::Top);
    }

    #[test]
    fn test_lattice_meet_top() {
        let top = LatticeType::Top;
        let int_type = LatticeType::Concrete(Type::Int);
        
        assert_eq!(top.meet(&int_type), int_type);
        assert_eq!(int_type.meet(&top), int_type);
    }

    #[test]
    fn test_type_state_join() {
        let mut state1 = TypeState::new();
        state1.set("x".to_string(), LatticeType::Concrete(Type::Int));
        state1.set("y".to_string(), LatticeType::Concrete(Type::String));
        
        let mut state2 = TypeState::new();
        state2.set("x".to_string(), LatticeType::Concrete(Type::Int));
        state2.set("z".to_string(), LatticeType::Concrete(Type::Bool));
        
        let joined = state1.join(&state2);
        
        // x is in both, should be Int
        assert_eq!(joined.get("x"), LatticeType::Concrete(Type::Int));
        // y only in state1, joined with Bottom = String
        assert_eq!(joined.get("y"), LatticeType::Concrete(Type::String));
        // z only in state2, joined with Bottom = Bool
        assert_eq!(joined.get("z"), LatticeType::Concrete(Type::Bool));
    }

    #[test]
    fn test_type_state_conflicting_join() {
        let mut state1 = TypeState::new();
        state1.set("x".to_string(), LatticeType::Concrete(Type::Int));
        
        let mut state2 = TypeState::new();
        state2.set("x".to_string(), LatticeType::Concrete(Type::String));
        
        let joined = state1.join(&state2);
        
        // Incompatible types go to Top
        assert_eq!(joined.get("x"), LatticeType::Top);
    }

    #[test]
    fn test_binary_op_types() {
        // Arithmetic
        assert_eq!(TypeLattice::binary_op_type(crate::hir::BinOp::Add, &Type::Int, &Type::Int), Type::Int);
        assert_eq!(TypeLattice::binary_op_type(crate::hir::BinOp::Add, &Type::Float, &Type::Int), Type::Float);
        
        // Division
        assert_eq!(TypeLattice::binary_op_type(crate::hir::BinOp::Div, &Type::Int, &Type::Int), Type::Float);
        assert_eq!(TypeLattice::binary_op_type(crate::hir::BinOp::FloorDiv, &Type::Int, &Type::Int), Type::Int);
        
        // Comparison
        assert_eq!(TypeLattice::binary_op_type(crate::hir::BinOp::Eq, &Type::Int, &Type::Int), Type::Bool);
        assert_eq!(TypeLattice::binary_op_type(crate::hir::BinOp::Lt, &Type::String, &Type::String), Type::Bool);
    }

    #[test]
    fn test_unary_op_types() {
        assert_eq!(TypeLattice::unary_op_type(crate::hir::UnaryOp::Not, &Type::Bool), Type::Bool);
        assert_eq!(TypeLattice::unary_op_type(crate::hir::UnaryOp::Neg, &Type::Int), Type::Int);
        assert_eq!(TypeLattice::unary_op_type(crate::hir::UnaryOp::BitNot, &Type::Int), Type::Int);
    }

    #[test]
    fn test_element_type() {
        assert_eq!(TypeLattice::element_type(&Type::List(Box::new(Type::Int))), Type::Int);
        assert_eq!(TypeLattice::element_type(&Type::String), Type::String);
        assert_eq!(TypeLattice::element_type(&Type::Dict(Box::new(Type::String), Box::new(Type::Int))), Type::Int);
    }

    #[test]
    fn test_optional_type_join() {
        let int_type = LatticeType::Concrete(Type::Int);
        let none_type = LatticeType::Concrete(Type::None);
        
        let result = int_type.join(&none_type);
        assert_eq!(result, LatticeType::Concrete(Type::Optional(Box::new(Type::Int))));
    }

    #[test]
    fn test_list_type_join() {
        let list1 = LatticeType::Concrete(Type::List(Box::new(Type::Int)));
        let list2 = LatticeType::Concrete(Type::List(Box::new(Type::Int)));
        
        let result = list1.join(&list2);
        assert_eq!(result, LatticeType::Concrete(Type::List(Box::new(Type::Int))));
    }
}
