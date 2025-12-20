use crate::hir::{AssignTarget, HirExpr, HirFunction, HirStmt, Type};
use std::collections::{HashMap, HashSet};

/// Tracks how parameters are used within a function to infer borrowing patterns
#[derive(Debug, Default)]
pub struct BorrowingContext {
    /// Parameters that are mutated in the function
    mutated_params: HashSet<String>,
    /// Parameters that escape (are returned or stored)
    escaping_params: HashSet<String>,
    /// Parameters that are only read
    read_only_params: HashSet<String>,
    /// Parameters used in loops (may need special handling)
    loop_used_params: HashSet<String>,
    /// Maps loop variables to the parameter they originate from
    /// e.g., "item" -> "state" when iterating over state.items
    loop_var_origins: HashMap<String, String>,
}

/// Analysis result for a single parameter
#[derive(Debug, Clone, PartialEq)]
pub enum BorrowingPattern {
    /// Parameter should be taken by value (moved)
    Owned,
    /// Parameter can be borrowed immutably
    Borrowed,
    /// Parameter needs mutable borrow
    MutableBorrow,
}

impl BorrowingContext {
    pub fn new() -> Self {
        Self::default()
    }

    /// Analyze a function to determine parameter borrowing patterns
    pub fn analyze_function(&mut self, func: &HirFunction) {
        // First pass: identify all parameters
        for param in &func.params {
            self.read_only_params.insert(param.name.clone());
        }

        // Analyze function body
        for stmt in &func.body {
            self.analyze_stmt(stmt);
        }

        // Remove read-only classification from mutated or escaping params
        for param in &self.mutated_params {
            self.read_only_params.remove(param);
        }
        for param in &self.escaping_params {
            self.read_only_params.remove(param);
        }
    }

    /// Get the borrowing pattern for a specific parameter
    pub fn get_pattern(&self, param_name: &str, param_type: &Type) -> BorrowingPattern {
        if self.escaping_params.contains(param_name) {
            // Parameters that escape must be owned
            BorrowingPattern::Owned
        } else if self.mutated_params.contains(param_name) {
            // Mutated parameters need mutable borrow
            BorrowingPattern::MutableBorrow
        } else if self.is_copyable(param_type) {
            // Small copyable types should be passed by value
            BorrowingPattern::Owned
        } else {
            // Everything else can be borrowed
            BorrowingPattern::Borrowed
        }
    }

    /// Generate Rust parameter signature based on borrowing pattern
    pub fn generate_param_signature(&self, param_name: &str, param_type: &Type) -> String {
        let pattern = self.get_pattern(param_name, param_type);
        let type_str = self.type_to_rust_string(param_type);

        match pattern {
            BorrowingPattern::Owned => format!("{}: {}", param_name, type_str),
            BorrowingPattern::Borrowed => format!("{}: &{}", param_name, type_str),
            BorrowingPattern::MutableBorrow => format!("{}: &mut {}", param_name, type_str),
        }
    }

    fn analyze_stmt(&mut self, stmt: &HirStmt) {
        match stmt {
            HirStmt::Assign { target, value, .. } => self.analyze_assign(target, value),
            HirStmt::Return(Some(expr)) => self.analyze_return(expr),
            HirStmt::Expr(expr) => self.analyze_expr(expr),
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => self.analyze_if(condition, then_body, else_body),
            HirStmt::While { condition, body } => self.analyze_while(condition, body),
            HirStmt::For { target, iter, body } => self.analyze_for(target, iter, body),
            _ => {}
        }
    }

    fn analyze_assign(&mut self, target: &AssignTarget, value: &HirExpr) {
        match target {
            AssignTarget::Symbol(symbol) => {
                if self.read_only_params.contains(symbol) {
                    self.mutated_params.insert(symbol.clone());
                }
            }
            AssignTarget::Attribute { value: obj, .. } => {
                // Check if we're mutating an attribute of a loop variable
                // e.g., item.value = ... where item comes from state.items
                if let HirExpr::Var(var_name) = obj.as_ref() {
                    if let Some(param_name) = self.loop_var_origins.get(var_name) {
                        // Mark the original parameter as mutated
                        self.mutated_params.insert(param_name.clone());
                    }
                }
            }
            AssignTarget::Index { base, .. } => {
                // Similarly handle index assignments through loop variables
                if let HirExpr::Var(var_name) = base.as_ref() {
                    if let Some(param_name) = self.loop_var_origins.get(var_name) {
                        self.mutated_params.insert(param_name.clone());
                    }
                }
            }
            _ => {}
        }
        self.check_escaping_expr(value);
        self.analyze_expr(value);
    }

    fn analyze_return(&mut self, expr: &HirExpr) {
        self.check_escaping_expr(expr);
        self.analyze_expr(expr);
    }

    fn analyze_if(
        &mut self,
        condition: &HirExpr,
        then_body: &[HirStmt],
        else_body: &Option<Vec<HirStmt>>,
    ) {
        self.analyze_expr(condition);
        for stmt in then_body {
            self.analyze_stmt(stmt);
        }
        if let Some(else_stmts) = else_body {
            for stmt in else_stmts {
                self.analyze_stmt(stmt);
            }
        }
    }

    fn analyze_while(&mut self, condition: &HirExpr, body: &[HirStmt]) {
        self.analyze_expr(condition);
        self.mark_loop_params(body);
        for stmt in body {
            self.analyze_stmt(stmt);
        }
    }

    fn analyze_for(&mut self, target: &AssignTarget, iter: &HirExpr, body: &[HirStmt]) {
        // Track the origin of the loop variable
        // If we're iterating over param.attr (e.g., state.items), track that
        if let AssignTarget::Symbol(loop_var) = target {
            if let Some(param_name) = self.extract_param_from_expr(iter) {
                self.loop_var_origins.insert(loop_var.clone(), param_name);
            }
        }

        self.analyze_expr(iter);
        self.mark_loop_params(body);
        for stmt in body {
            self.analyze_stmt(stmt);
        }
    }

    #[allow(clippy::only_used_in_recursion)]
    fn analyze_expr(&mut self, expr: &HirExpr) {
        match expr {
            HirExpr::Binary { op: _, left, right } => self.analyze_binary(left, right),
            HirExpr::Unary { op: _, operand } => self.analyze_expr(operand),
            HirExpr::Call { func: _, args, .. } => self.analyze_call(args),
            HirExpr::List(elts) | HirExpr::Tuple(elts) => self.analyze_collection(elts),
            HirExpr::Dict(items) => self.analyze_dict(items),
            HirExpr::Index { base, index } => self.analyze_index(base, index),
            _ => {}
        }
    }

    fn analyze_binary(&mut self, left: &HirExpr, right: &HirExpr) {
        self.analyze_expr(left);
        self.analyze_expr(right);
    }

    fn analyze_call(&mut self, args: &[HirExpr]) {
        for arg in args {
            self.analyze_expr(arg);
        }
    }

    fn analyze_collection(&mut self, elts: &[HirExpr]) {
        for elt in elts {
            self.analyze_expr(elt);
        }
    }

    fn analyze_dict(&mut self, items: &[(HirExpr, HirExpr)]) {
        for (k, v) in items {
            self.analyze_expr(k);
            self.analyze_expr(v);
        }
    }

    fn analyze_index(&mut self, base: &HirExpr, index: &HirExpr) {
        self.analyze_expr(base);
        self.analyze_expr(index);
    }

    fn check_escaping_expr(&mut self, expr: &HirExpr) {
        match expr {
            HirExpr::Var(name) => {
                // Direct return of parameter
                self.escaping_params.insert(name.clone());
            }
            HirExpr::List(elts) | HirExpr::Tuple(elts) => {
                // Parameters in collections that are returned
                for elt in elts {
                    if let HirExpr::Var(name) = elt {
                        self.escaping_params.insert(name.clone());
                    }
                }
            }
            _ => {}
        }
    }

    fn mark_loop_params(&mut self, body: &[HirStmt]) {
        for stmt in body {
            self.find_params_in_stmt(stmt);
        }
    }

    fn find_params_in_stmt(&mut self, stmt: &HirStmt) {
        match stmt {
            HirStmt::Expr(expr) => self.find_params_in_expr(expr),
            HirStmt::Assign { value, .. } => self.find_params_in_expr(value),
            _ => {}
        }
    }

    fn find_params_in_expr(&mut self, expr: &HirExpr) {
        match expr {
            HirExpr::Var(name) => {
                if self.read_only_params.contains(name)
                    || self.mutated_params.contains(name)
                    || self.escaping_params.contains(name)
                {
                    self.loop_used_params.insert(name.clone());
                }
            }
            // Recursively traverse binary expressions
            HirExpr::Binary { left, right, .. } => {
                self.find_params_in_expr(left);
                self.find_params_in_expr(right);
            }
            // Recursively traverse unary expressions
            HirExpr::Unary { operand, .. } => {
                self.find_params_in_expr(operand);
            }
            // Recursively traverse function call arguments
            HirExpr::Call { args, .. } => {
                for arg in args {
                    self.find_params_in_expr(arg);
                }
            }
            // Recursively traverse collections
            HirExpr::List(elts) | HirExpr::Tuple(elts) | HirExpr::Set(elts) => {
                for elt in elts {
                    self.find_params_in_expr(elt);
                }
            }
            // Recursively traverse dict items
            HirExpr::Dict(items) => {
                for (key, value) in items {
                    self.find_params_in_expr(key);
                    self.find_params_in_expr(value);
                }
            }
            // Recursively traverse index expressions
            HirExpr::Index { base, index } => {
                self.find_params_in_expr(base);
                self.find_params_in_expr(index);
            }
            _ => {}
        }
    }

    /// Extract the parameter name from an expression
    /// For example, from `state.items` extract `state`
    fn extract_param_from_expr(&self, expr: &HirExpr) -> Option<String> {
        match expr {
            HirExpr::Var(name) => {
                // Direct parameter reference
                if self.read_only_params.contains(name)
                    || self.mutated_params.contains(name)
                    || self.escaping_params.contains(name)
                {
                    Some(name.clone())
                } else {
                    None
                }
            }
            HirExpr::Attribute { value, .. } => {
                // Attribute access like state.items - extract the base
                self.extract_param_from_expr(value)
            }
            HirExpr::Index { base, .. } => {
                // Index access like state[0] - extract the base
                self.extract_param_from_expr(base)
            }
            _ => None,
        }
    }

    fn is_copyable(&self, ty: &Type) -> bool {
        matches!(ty, Type::Int | Type::Float | Type::Bool | Type::None)
    }

    #[allow(clippy::only_used_in_recursion)]
    fn type_to_rust_string(&self, ty: &Type) -> String {
        match ty {
            Type::Unknown | Type::Int | Type::Float | Type::String | Type::Bool | Type::None => {
                self.primitive_type_to_rust(ty)
            }
            Type::List(_) | Type::Dict(_, _) | Type::Set(_) | Type::Array { .. } => {
                self.collection_type_to_rust(ty)
            }
            Type::Tuple(types) => self.tuple_type_to_rust(types),
            Type::Optional(inner) => self.optional_type_to_rust(inner),
            Type::Custom(name) | Type::TypeVar(name) => name.clone(),
            Type::Generic { base, .. } => base.clone(),
            Type::Function { .. } => "/* function */".to_string(),
            Type::Union(_) => "Union".to_string(),
            Type::Final(inner) => self.type_to_rust_string(inner), // Unwrap Final to get the actual type
        }
    }

    fn primitive_type_to_rust(&self, ty: &Type) -> String {
        match ty {
            Type::Unknown => "serde_json::Value".to_string(),
            Type::Int => "i32".to_string(),
            Type::Float => "f64".to_string(),
            Type::String => "String".to_string(),
            Type::Bool => "bool".to_string(),
            Type::None => "()".to_string(),
            _ => unreachable!("primitive_type_to_rust called with non-primitive type"),
        }
    }

    fn collection_type_to_rust(&self, ty: &Type) -> String {
        match ty {
            Type::List(inner) => self.list_type_to_rust(inner),
            Type::Dict(k, v) => self.dict_type_to_rust(k, v),
            Type::Set(element) => self.set_type_to_rust(element),
            Type::Array { element_type, .. } => self.array_type_to_rust(element_type),
            _ => unreachable!("collection_type_to_rust called with non-collection type"),
        }
    }

    fn list_type_to_rust(&self, inner: &Type) -> String {
        format!("Vec<{}>", self.type_to_rust_string(inner))
    }

    fn set_type_to_rust(&self, element: &Type) -> String {
        format!("HashSet<{}>", self.type_to_rust_string(element))
    }

    fn array_type_to_rust(&self, element_type: &Type) -> String {
        format!("Array<{}>", self.type_to_rust_string(element_type))
    }

    fn dict_type_to_rust(&self, k: &Type, v: &Type) -> String {
        format!(
            "HashMap<{}, {}>",
            self.type_to_rust_string(k),
            self.type_to_rust_string(v)
        )
    }

    fn tuple_type_to_rust(&self, types: &[Type]) -> String {
        if types.is_empty() {
            "()".to_string()
        } else {
            let type_strs: Vec<String> =
                types.iter().map(|t| self.type_to_rust_string(t)).collect();
            format!("({})", type_strs.join(", "))
        }
    }

    fn optional_type_to_rust(&self, inner: &Type) -> String {
        format!("Option<{}>", self.type_to_rust_string(inner))
    }
}
