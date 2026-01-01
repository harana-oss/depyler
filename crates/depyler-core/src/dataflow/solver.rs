//! Dataflow analysis framework with worklist-based fixpoint solver

use super::cfg::{BasicBlock, BlockId, Cfg, CfgStmt, Terminator};
use super::lattice::{LatticeType, TypeLattice, TypeState};
use super::mutations::MutationRegistry;
use crate::hir::{HirExpr, Type};
use std::collections::{HashMap, HashSet, VecDeque};

/// Direction of dataflow analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataflowDirection {
    Forward,
    Backward,
}

/// Generic trait for dataflow analyses
pub trait DataflowAnalysis {
    /// The type of facts being propagated
    type Fact: Clone + PartialEq;

    /// Direction of the analysis
    fn direction(&self) -> DataflowDirection;

    /// Initial fact for entry (forward) or exit (backward) block
    fn initial_fact(&self) -> Self::Fact;

    /// Bottom element for the lattice
    fn bottom(&self) -> Self::Fact;

    /// Join/merge facts from multiple predecessors
    fn join(&self, facts: &[Self::Fact]) -> Self::Fact;

    /// Transfer function: compute output fact from input fact for a block
    fn transfer(&self, block: &BasicBlock, input: &Self::Fact) -> Self::Fact;
}

/// Result of fixpoint computation
#[derive(Debug)]
pub struct FixpointResult<F> {
    /// Facts at entry of each block
    pub in_facts: HashMap<BlockId, F>,
    /// Facts at exit of each block  
    pub out_facts: HashMap<BlockId, F>,
    /// Number of iterations to reach fixpoint
    pub iterations: usize,
}

/// Worklist-based fixpoint solver
pub struct FixpointSolver;

impl FixpointSolver {
    /// Compute fixpoint for a dataflow analysis
    pub fn solve<A: DataflowAnalysis>(analysis: &A, cfg: &Cfg) -> FixpointResult<A::Fact> {
        match analysis.direction() {
            DataflowDirection::Forward => Self::solve_forward(analysis, cfg),
            DataflowDirection::Backward => Self::solve_backward(analysis, cfg),
        }
    }

    fn solve_forward<A: DataflowAnalysis>(analysis: &A, cfg: &Cfg) -> FixpointResult<A::Fact> {
        let mut in_facts: HashMap<BlockId, A::Fact> = HashMap::new();
        let mut out_facts: HashMap<BlockId, A::Fact> = HashMap::new();

        // Initialize all blocks with bottom
        for &block_id in cfg.blocks.keys() {
            in_facts.insert(block_id, analysis.bottom());
            out_facts.insert(block_id, analysis.bottom());
        }

        // Entry block gets initial fact
        in_facts.insert(cfg.entry, analysis.initial_fact());

        // Initialize worklist with blocks in reverse postorder
        let mut worklist: VecDeque<BlockId> = cfg.reverse_postorder().into_iter().collect();
        let mut in_worklist: HashSet<BlockId> = worklist.iter().copied().collect();

        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 1000;

        while let Some(block_id) = worklist.pop_front() {
            in_worklist.remove(&block_id);
            iterations += 1;

            if iterations > MAX_ITERATIONS {
                break; // Prevent infinite loops
            }

            let block = match cfg.blocks.get(&block_id) {
                Some(b) => b,
                None => continue,
            };

            // Compute input by joining predecessor outputs
            let pred_facts: Vec<A::Fact> = block
                .predecessors
                .iter()
                .filter_map(|pred_id| out_facts.get(pred_id).cloned())
                .collect();

            let new_in = if pred_facts.is_empty() {
                if block_id == cfg.entry {
                    analysis.initial_fact()
                } else {
                    analysis.bottom()
                }
            } else {
                analysis.join(&pred_facts)
            };

            // Apply transfer function
            let new_out = analysis.transfer(block, &new_in);

            // Check if output changed
            let old_out = out_facts.get(&block_id);
            let changed = old_out.map_or(true, |old| old != &new_out);

            if changed {
                in_facts.insert(block_id, new_in);
                out_facts.insert(block_id, new_out);

                // Add successors to worklist
                for &succ_id in &block.successors {
                    if !in_worklist.contains(&succ_id) {
                        worklist.push_back(succ_id);
                        in_worklist.insert(succ_id);
                    }
                }
            }
        }

        FixpointResult {
            in_facts,
            out_facts,
            iterations,
        }
    }

    fn solve_backward<A: DataflowAnalysis>(analysis: &A, cfg: &Cfg) -> FixpointResult<A::Fact> {
        let mut in_facts: HashMap<BlockId, A::Fact> = HashMap::new();
        let mut out_facts: HashMap<BlockId, A::Fact> = HashMap::new();

        // Initialize all blocks with bottom
        for &block_id in cfg.blocks.keys() {
            in_facts.insert(block_id, analysis.bottom());
            out_facts.insert(block_id, analysis.bottom());
        }

        // Exit block gets initial fact
        out_facts.insert(cfg.exit, analysis.initial_fact());

        // Initialize worklist with blocks in postorder
        let mut worklist: VecDeque<BlockId> = cfg.postorder().into_iter().collect();
        let mut in_worklist: HashSet<BlockId> = worklist.iter().copied().collect();

        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 1000;

        while let Some(block_id) = worklist.pop_front() {
            in_worklist.remove(&block_id);
            iterations += 1;

            if iterations > MAX_ITERATIONS {
                break;
            }

            let block = match cfg.blocks.get(&block_id) {
                Some(b) => b,
                None => continue,
            };

            // Compute output by joining successor inputs
            let succ_facts: Vec<A::Fact> = block
                .successors
                .iter()
                .filter_map(|succ_id| in_facts.get(succ_id).cloned())
                .collect();

            let new_out = if succ_facts.is_empty() {
                if block_id == cfg.exit {
                    analysis.initial_fact()
                } else {
                    analysis.bottom()
                }
            } else {
                analysis.join(&succ_facts)
            };

            // Apply transfer function (backward)
            let new_in = analysis.transfer(block, &new_out);

            // Check if input changed
            let old_in = in_facts.get(&block_id);
            let changed = old_in.map_or(true, |old| old != &new_in);

            if changed {
                out_facts.insert(block_id, new_out);
                in_facts.insert(block_id, new_in);

                // Add predecessors to worklist
                for &pred_id in &block.predecessors {
                    if !in_worklist.contains(&pred_id) {
                        worklist.push_back(pred_id);
                        in_worklist.insert(pred_id);
                    }
                }
            }
        }

        FixpointResult {
            in_facts,
            out_facts,
            iterations,
        }
    }
}

/// Forward type propagation analysis
pub struct TypePropagation {
    /// Initial parameter types
    initial_types: HashMap<String, Type>,
    /// Built-in function signatures
    builtins: HashMap<String, Type>,
    /// User-defined function return types from the same module
    user_functions: HashMap<String, Type>,
    /// Modular mutation handlers for container type inference
    mutation_registry: MutationRegistry,
}

impl TypePropagation {
    pub fn new(param_types: HashMap<String, Type>) -> Self {
        let mut builtins = HashMap::new();

        // Common built-in function return types
        builtins.insert("len".to_string(), Type::Int);
        builtins.insert("range".to_string(), Type::Custom("range".to_string()));
        builtins.insert("int".to_string(), Type::Int);
        builtins.insert("float".to_string(), Type::Float);
        builtins.insert("str".to_string(), Type::String);
        builtins.insert("bool".to_string(), Type::Bool);
        builtins.insert("list".to_string(), Type::List(Box::new(Type::Unknown)));
        builtins.insert(
            "dict".to_string(),
            Type::Dict(Box::new(Type::Unknown), Box::new(Type::Unknown)),
        );
        builtins.insert("set".to_string(), Type::Set(Box::new(Type::Unknown)));
        builtins.insert("abs".to_string(), Type::Unknown); // Depends on input
        builtins.insert("min".to_string(), Type::Unknown);
        builtins.insert("max".to_string(), Type::Unknown);
        builtins.insert("sum".to_string(), Type::Unknown);
        builtins.insert("sorted".to_string(), Type::List(Box::new(Type::Unknown)));
        builtins.insert("reversed".to_string(), Type::List(Box::new(Type::Unknown)));
        builtins.insert(
            "enumerate".to_string(),
            Type::List(Box::new(Type::Tuple(vec![Type::Int, Type::Unknown]))),
        );
        builtins.insert(
            "zip".to_string(),
            Type::List(Box::new(Type::Tuple(vec![Type::Unknown, Type::Unknown]))),
        );
        builtins.insert("map".to_string(), Type::List(Box::new(Type::Unknown)));
        builtins.insert("filter".to_string(), Type::List(Box::new(Type::Unknown)));
        builtins.insert("print".to_string(), Type::None);
        builtins.insert("input".to_string(), Type::String);
        builtins.insert("open".to_string(), Type::Custom("file".to_string()));

        // Math functions
        builtins.insert("round".to_string(), Type::Int);
        builtins.insert("pow".to_string(), Type::Unknown); // Depends on input
        builtins.insert(
            "divmod".to_string(),
            Type::Tuple(vec![Type::Int, Type::Int]),
        );

        // String functions (standalone, not methods)
        builtins.insert("upper".to_string(), Type::String);
        builtins.insert("lower".to_string(), Type::String);
        builtins.insert("ord".to_string(), Type::Int);
        builtins.insert("chr".to_string(), Type::String);
        builtins.insert("repr".to_string(), Type::String);
        builtins.insert("ascii".to_string(), Type::String);

        // Numeric conversion functions
        builtins.insert("hex".to_string(), Type::String);
        builtins.insert("oct".to_string(), Type::String);
        builtins.insert("bin".to_string(), Type::String);

        // Type checking functions
        builtins.insert("type".to_string(), Type::Custom("type".to_string()));
        builtins.insert("isinstance".to_string(), Type::Bool);
        builtins.insert("issubclass".to_string(), Type::Bool);
        builtins.insert("callable".to_string(), Type::Bool);
        builtins.insert("hasattr".to_string(), Type::Bool);
        builtins.insert("getattr".to_string(), Type::Unknown);
        builtins.insert("setattr".to_string(), Type::None);
        builtins.insert("delattr".to_string(), Type::None);

        // Collection functions
        builtins.insert("any".to_string(), Type::Bool);
        builtins.insert("all".to_string(), Type::Bool);
        builtins.insert("tuple".to_string(), Type::Tuple(vec![Type::Unknown]));
        builtins.insert("frozenset".to_string(), Type::Set(Box::new(Type::Unknown)));

        // Other common functions
        builtins.insert("id".to_string(), Type::Int);
        builtins.insert("hash".to_string(), Type::Int);
        builtins.insert("iter".to_string(), Type::Custom("iterator".to_string()));
        builtins.insert("next".to_string(), Type::Unknown);
        builtins.insert("format".to_string(), Type::String);

        Self {
            initial_types: param_types,
            builtins,
            user_functions: HashMap::new(),
            mutation_registry: MutationRegistry::new(),
        }
    }

    /// Set user-defined function signatures for interprocedural analysis
    pub fn with_user_functions(mut self, functions: HashMap<String, Type>) -> Self {
        self.user_functions = functions;
        self
    }

    /// Infer type of an expression given current type state
    pub fn infer_expr_type(&self, expr: &HirExpr, state: &TypeState) -> Type {
        match expr {
            HirExpr::Literal(lit) => TypeLattice::type_of_literal(lit),
            HirExpr::Var(name) => state.get(name).to_hir_type(),
            HirExpr::Binary { op, left, right } => {
                let left_ty = self.infer_expr_type(left, state);
                let right_ty = self.infer_expr_type(right, state);
                TypeLattice::binary_op_type(*op, &left_ty, &right_ty)
            }
            HirExpr::Unary { op, operand } => {
                let operand_ty = self.infer_expr_type(operand, state);
                TypeLattice::unary_op_type(*op, &operand_ty)
            }
            HirExpr::Call { func, args, .. } => self.infer_call_type(func, args, state),
            HirExpr::MethodCall {
                object,
                method,
                args,
                ..
            } => self.infer_method_call_type(object, method, args, state),
            HirExpr::Index { base, .. } => {
                let base_ty = self.infer_expr_type(base, state);
                TypeLattice::element_type(&base_ty)
            }
            HirExpr::Slice { base, .. } => {
                // Slice returns same type as base
                self.infer_expr_type(base, state)
            }
            HirExpr::Attribute { value, attr } => self.infer_attribute_type(value, attr, state),
            HirExpr::List(elems) => {
                if elems.is_empty() {
                    Type::List(Box::new(Type::Unknown))
                } else {
                    let elem_ty = self.infer_expr_type(&elems[0], state);
                    Type::List(Box::new(elem_ty))
                }
            }
            HirExpr::Dict(items) => {
                if items.is_empty() {
                    Type::Dict(Box::new(Type::Unknown), Box::new(Type::Unknown))
                } else {
                    let (k, v) = &items[0];
                    let key_ty = self.infer_expr_type(k, state);
                    let val_ty = self.infer_expr_type(v, state);
                    Type::Dict(Box::new(key_ty), Box::new(val_ty))
                }
            }
            HirExpr::Tuple(elems) => {
                let types: Vec<Type> = elems
                    .iter()
                    .map(|e| self.infer_expr_type(e, state))
                    .collect();
                Type::Tuple(types) // Treat Uninitialized as Unknown
            }
            HirExpr::Uninitialized => Type::Unknown,
            HirExpr::Set(elems) => {
                if elems.is_empty() {
                    Type::Set(Box::new(Type::Unknown))
                } else {
                    let elem_ty = self.infer_expr_type(&elems[0], state);
                    Type::Set(Box::new(elem_ty))
                }
            }
            HirExpr::ListComp { element, .. } => {
                // For comprehensions, we'd need to analyze the iteration
                Type::List(Box::new(self.infer_expr_type(element, state)))
            }
            HirExpr::FlattenedListComp { element, .. } => {
                Type::List(Box::new(self.infer_expr_type(element, state)))
            }
            HirExpr::SetComp { element, .. } => {
                Type::Set(Box::new(self.infer_expr_type(element, state)))
            }
            HirExpr::DictComp { key, value, .. } => Type::Dict(
                Box::new(self.infer_expr_type(key, state)),
                Box::new(self.infer_expr_type(value, state)),
            ),
            HirExpr::Lambda { .. } => Type::Function {
                params: vec![Type::Unknown],
                ret: Box::new(Type::Unknown),
            },
            HirExpr::IfExpr { body, orelse, .. } => {
                // Join types from both branches
                let then_ty = self.infer_expr_type(body, state);
                let else_ty = self.infer_expr_type(orelse, state);
                let then_lattice = LatticeType::from_hir_type(&then_ty);
                let else_lattice = LatticeType::from_hir_type(&else_ty);
                then_lattice.join(&else_lattice).to_hir_type()
            }
            HirExpr::FString { .. } => Type::String,
            HirExpr::Await { value } => {
                // Await returns the inner type of the future
                self.infer_expr_type(value, state)
            }
            HirExpr::Yield { value } => value
                .as_ref()
                .map_or(Type::None, |v| self.infer_expr_type(v, state)),
            HirExpr::Borrow { expr, .. } => self.infer_expr_type(expr, state),
            HirExpr::FrozenSet(elems) => {
                if elems.is_empty() {
                    Type::Set(Box::new(Type::Unknown))
                } else {
                    let elem_ty = self.infer_expr_type(&elems[0], state);
                    Type::Set(Box::new(elem_ty))
                }
            }
            HirExpr::SortByKey { iterable, .. } => {
                // sorted returns a list
                let iter_ty = self.infer_expr_type(iterable, state);
                match iter_ty {
                    Type::List(elem) => Type::List(elem),
                    Type::Set(elem) => Type::List(elem),
                    _ => Type::List(Box::new(Type::Unknown)),
                }
            }
            HirExpr::GeneratorExp { element, .. } => Type::Custom("generator".to_string()),
            HirExpr::NamedExpr { value, .. } => {
                // Named expression returns the type of its value
                self.infer_expr_type(value, state)
            }
        }
    }

    fn infer_call_type(&self, func: &str, args: &[HirExpr], state: &TypeState) -> Type {
        // Special synthetic function for for-loop iteration
        if func == "__iter_next__" {
            if let Some(iter_expr) = args.first() {
                let iter_ty = self.infer_expr_type(iter_expr, state);
                return TypeLattice::element_type(&iter_ty);
            }
        }

        // Check builtins first
        if let Some(ret_ty) = self.builtins.get(func) {
            // Special handling for type-dependent builtins
            match func {
                "abs" | "min" | "max" | "pow" => {
                    if let Some(arg) = args.first() {
                        return self.infer_expr_type(arg, state);
                    }
                }
                "sum" => {
                    if let Some(arg) = args.first() {
                        let iter_ty = self.infer_expr_type(arg, state);
                        return TypeLattice::element_type(&iter_ty);
                    }
                }
                _ => {}
            }
            return ret_ty.clone();
        }

        // Check user-defined functions from the same module
        if let Some(ret_ty) = self.user_functions.get(func) {
            return ret_ty.clone();
        }

        // Check if it's a variable holding a callable
        let var_ty = state.get(func).to_hir_type();
        if let Type::Function { ret, .. } = var_ty {
            return *ret;
        }

        Type::Unknown
    }

    fn infer_method_call_type(
        &self,
        object: &HirExpr,
        method: &str,
        _args: &[HirExpr],
        state: &TypeState,
    ) -> Type {
        let obj_ty = self.infer_expr_type(object, state);

        match (&obj_ty, method) {
            // String methods
            (
                Type::String,
                "upper" | "lower" | "strip" | "lstrip" | "rstrip" | "title" | "capitalize",
            ) => Type::String,
            (Type::String, "split" | "splitlines") => Type::List(Box::new(Type::String)),
            (Type::String, "join") => Type::String,
            (Type::String, "find" | "rfind" | "index" | "rindex" | "count") => Type::Int,
            (
                Type::String,
                "startswith" | "endswith" | "isalpha" | "isdigit" | "isalnum" | "isspace",
            ) => Type::Bool,
            (Type::String, "replace" | "format") => Type::String,
            (Type::String, "encode") => Type::Custom("bytes".to_string()),

            // List methods
            (
                Type::List(elem),
                "append" | "extend" | "insert" | "remove" | "clear" | "reverse" | "sort",
            ) => Type::None,
            (Type::List(elem), "pop") => *elem.clone(),
            (Type::List(_), "index" | "count") => Type::Int,
            (Type::List(elem), "copy") => Type::List(elem.clone()),

            // Dict methods
            (Type::Dict(_, v), "get" | "pop" | "setdefault") => Type::Optional(v.clone()),
            (Type::Dict(k, _), "keys") => Type::List(k.clone()),
            (Type::Dict(_, v), "values") => Type::List(v.clone()),
            (Type::Dict(k, v), "items") => {
                Type::List(Box::new(Type::Tuple(vec![*k.clone(), *v.clone()])))
            }
            (Type::Dict(_, _), "update" | "clear") => Type::None,
            (Type::Dict(k, v), "copy") => Type::Dict(k.clone(), v.clone()),

            // Set methods
            (Type::Set(elem), "add" | "remove" | "discard" | "clear" | "update") => Type::None,
            (Type::Set(elem), "pop") => *elem.clone(),
            (
                Type::Set(elem),
                "copy" | "union" | "intersection" | "difference" | "symmetric_difference",
            ) => Type::Set(elem.clone()),
            (Type::Set(_), "issubset" | "issuperset" | "isdisjoint") => Type::Bool,

            _ => Type::Unknown,
        }
    }

    fn infer_attribute_type(&self, _value: &HirExpr, _attr: &str, _state: &TypeState) -> Type {
        // Would need class information for proper inference
        Type::Unknown
    }

    /// Infer the element type from an iterator expression (for for-loops)
    pub fn infer_iterator_element_type(&self, iter: &HirExpr, state: &TypeState) -> Type {
        let iter_ty = self.infer_expr_type(iter, state);

        match &iter_ty {
            Type::List(elem) => *elem.clone(),
            Type::Set(elem) => *elem.clone(),
            Type::Tuple(elems) if !elems.is_empty() => elems[0].clone(),
            Type::Dict(key, _) => *key.clone(),
            Type::String => Type::String,
            Type::Custom(name) if name == "range" => Type::Int,
            _ => Type::Unknown,
        }
    }

    /// Apply type mutation from index assignment (e.g., dict[k] = v, list[i] = v)
    fn apply_index_assign_mutation(
        &self,
        state: &mut TypeState,
        base: &str,
        index: &HirExpr,
        value: &HirExpr,
    ) {
        let base_ty = state.get(base).to_hir_type();
        let index_ty = self.infer_expr_type(index, state);
        let value_ty = self.infer_expr_type(value, state);

        let new_ty = match &base_ty {
            Type::List(elem) => {
                // Refine list element type
                let joined =
                    LatticeType::from_hir_type(elem).join(&LatticeType::from_hir_type(&value_ty));
                Some(Type::List(Box::new(joined.to_hir_type())))
            }
            Type::Dict(key, val) => {
                // Refine dict key and value types
                let joined_key =
                    LatticeType::from_hir_type(key).join(&LatticeType::from_hir_type(&index_ty));
                let joined_val =
                    LatticeType::from_hir_type(val).join(&LatticeType::from_hir_type(&value_ty));
                Some(Type::Dict(
                    Box::new(joined_key.to_hir_type()),
                    Box::new(joined_val.to_hir_type()),
                ))
            }
            Type::Unknown => {
                // Infer type from usage: dict[k] = v suggests Dict type
                Some(Type::Dict(Box::new(index_ty), Box::new(value_ty)))
            }
            _ => None, // No refinement possible
        };

        if let Some(ty) = new_ty {
            if ty != base_ty {
                state.set(base.to_string(), LatticeType::from_hir_type(&ty));
            }
        }
    }

    /// Apply type mutation from expression statements (method calls)
    fn apply_expr_mutation(&self, state: &mut TypeState, expr: &HirExpr) {
        if let HirExpr::MethodCall {
            object,
            method,
            args,
            ..
        } = expr
        {
            // Extract the base variable name if this is a simple variable
            if let HirExpr::Var(var_name) = object.as_ref() {
                let current_ty = state.get(var_name).to_hir_type();

                if let Some(new_ty) = self.compute_mutation_type(&current_ty, method, args, state) {
                    state.set(var_name.clone(), LatticeType::from_hir_type(&new_ty));
                }
            }
        }
    }

    /// Compute the new type after a mutating method call
    fn compute_mutation_type(
        &self,
        current_ty: &Type,
        method: &str,
        args: &[HirExpr],
        state: &TypeState,
    ) -> Option<Type> {
        // Delegate to the modular mutation registry
        let infer_fn = |expr: &HirExpr, st: &TypeState| self.infer_expr_type(expr, st);
        self.mutation_registry
            .compute_mutation_type(current_ty, method, args, state, &infer_fn)
    }
}

impl DataflowAnalysis for TypePropagation {
    type Fact = TypeState;

    fn direction(&self) -> DataflowDirection {
        DataflowDirection::Forward
    }

    fn initial_fact(&self) -> TypeState {
        let mut state = TypeState::new();
        for (name, ty) in &self.initial_types {
            state.set(name.clone(), LatticeType::from_hir_type(ty));
        }
        state
    }

    fn bottom(&self) -> TypeState {
        TypeState::bottom()
    }

    fn join(&self, facts: &[TypeState]) -> TypeState {
        if facts.is_empty() {
            return TypeState::bottom();
        }
        let mut result = facts[0].clone();
        for fact in &facts[1..] {
            result = result.join(fact);
        }
        result
    }

    fn transfer(&self, block: &BasicBlock, input: &TypeState) -> TypeState {
        let mut state = input.clone();

        for stmt in &block.stmts {
            match stmt {
                CfgStmt::Assign {
                    target,
                    value,
                    type_annotation,
                } => {
                    let ty = if let Some(ann) = type_annotation {
                        // Explicit annotation takes precedence
                        LatticeType::from_hir_type(ann)
                    } else {
                        // Infer from value
                        let inferred = self.infer_expr_type(value, &state);
                        LatticeType::from_hir_type(&inferred)
                    };
                    state.set(target.clone(), ty);
                }
                CfgStmt::IndexAssign { base, index, value } => {
                    // Track type refinement from index assignment
                    self.apply_index_assign_mutation(&mut state, base, index, value);
                }
                CfgStmt::Expr(expr) => {
                    // Track mutations from method calls like append, add, update, etc.
                    self.apply_expr_mutation(&mut state, expr);
                }
                CfgStmt::Assert { .. } | CfgStmt::Pass => {
                    // No effect on types
                }
            }
        }

        // Handle terminator for additional type info (e.g., from for-loop headers)
        if let Some(term) = &block.terminator {
            if let Terminator::Loop { .. } = term {
                // Loop variables were already handled in CFG construction
            }
        }

        state
    }
}
