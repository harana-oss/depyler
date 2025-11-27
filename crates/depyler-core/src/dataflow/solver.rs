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

        Self {
            initial_types: param_types,
            builtins,
            mutation_registry: MutationRegistry::new(),
        }
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
                object, method, args, ..
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
                let types: Vec<Type> = elems.iter().map(|e| self.infer_expr_type(e, state)).collect();
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
            HirExpr::SetComp { element, .. } => Type::Set(Box::new(self.infer_expr_type(element, state))),
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
            HirExpr::Yield { value } => value.as_ref().map_or(Type::None, |v| self.infer_expr_type(v, state)),
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
                "abs" | "min" | "max" => {
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

        // Check if it's a variable holding a callable
        let var_ty = state.get(func).to_hir_type();
        if let Type::Function { ret, .. } = var_ty {
            return *ret;
        }

        Type::Unknown
    }

    fn infer_method_call_type(&self, object: &HirExpr, method: &str, _args: &[HirExpr], state: &TypeState) -> Type {
        let obj_ty = self.infer_expr_type(object, state);

        match (&obj_ty, method) {
            // String methods
            (Type::String, "upper" | "lower" | "strip" | "lstrip" | "rstrip" | "title" | "capitalize") => Type::String,
            (Type::String, "split" | "splitlines") => Type::List(Box::new(Type::String)),
            (Type::String, "join") => Type::String,
            (Type::String, "find" | "rfind" | "index" | "rindex" | "count") => Type::Int,
            (Type::String, "startswith" | "endswith" | "isalpha" | "isdigit" | "isalnum" | "isspace") => Type::Bool,
            (Type::String, "replace" | "format") => Type::String,
            (Type::String, "encode") => Type::Custom("bytes".to_string()),

            // List methods
            (Type::List(elem), "append" | "extend" | "insert" | "remove" | "clear" | "reverse" | "sort") => Type::None,
            (Type::List(elem), "pop") => *elem.clone(),
            (Type::List(_), "index" | "count") => Type::Int,
            (Type::List(elem), "copy") => Type::List(elem.clone()),

            // Dict methods
            (Type::Dict(_, v), "get" | "pop" | "setdefault") => Type::Optional(v.clone()),
            (Type::Dict(k, _), "keys") => Type::List(k.clone()),
            (Type::Dict(_, v), "values") => Type::List(v.clone()),
            (Type::Dict(k, v), "items") => Type::List(Box::new(Type::Tuple(vec![*k.clone(), *v.clone()]))),
            (Type::Dict(_, _), "update" | "clear") => Type::None,
            (Type::Dict(k, v), "copy") => Type::Dict(k.clone(), v.clone()),

            // Set methods
            (Type::Set(elem), "add" | "remove" | "discard" | "clear" | "update") => Type::None,
            (Type::Set(elem), "pop") => *elem.clone(),
            (Type::Set(elem), "copy" | "union" | "intersection" | "difference" | "symmetric_difference") => {
                Type::Set(elem.clone())
            }
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
    fn apply_index_assign_mutation(&self, state: &mut TypeState, base: &str, index: &HirExpr, value: &HirExpr) {
        let base_ty = state.get(base).to_hir_type();
        let index_ty = self.infer_expr_type(index, state);
        let value_ty = self.infer_expr_type(value, state);

        let new_ty = match &base_ty {
            Type::List(elem) => {
                // Refine list element type
                let joined = LatticeType::from_hir_type(elem).join(&LatticeType::from_hir_type(&value_ty));
                Some(Type::List(Box::new(joined.to_hir_type())))
            }
            Type::Dict(key, val) => {
                // Refine dict key and value types
                let joined_key = LatticeType::from_hir_type(key).join(&LatticeType::from_hir_type(&index_ty));
                let joined_val = LatticeType::from_hir_type(val).join(&LatticeType::from_hir_type(&value_ty));
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
            object, method, args, ..
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dataflow::cfg::CfgBuilder;
    use crate::hir::{AssignTarget, BinOp, FunctionProperties, HirParam, HirStmt, Literal};
    use depyler_annotations::TranspilationAnnotations;

    fn make_function(name: &str, params: Vec<HirParam>, body: Vec<HirStmt>) -> crate::hir::HirFunction {
        crate::hir::HirFunction {
            name: name.to_string(),
            params: smallvec::SmallVec::from_vec(params),
            ret_type: Type::Unknown,
            body,
            properties: FunctionProperties::default(),
            annotations: TranspilationAnnotations::default(),
            docstring: None,
        }
    }

    #[test]
    fn test_simple_assignment_propagation() {
        let func = make_function(
            "test",
            vec![],
            vec![
                HirStmt::Assign {
                    target: AssignTarget::Symbol("x".to_string()),
                    value: HirExpr::Literal(Literal::Int(42)),
                    type_annotation: None,
                },
                HirStmt::Assign {
                    target: AssignTarget::Symbol("y".to_string()),
                    value: HirExpr::Var("x".to_string()),
                    type_annotation: None,
                },
                HirStmt::Return(Some(HirExpr::Var("y".to_string()))),
            ],
        );

        let cfg = CfgBuilder::new().build_function(&func);
        let analysis = TypePropagation::new(HashMap::new());
        let result = FixpointSolver::solve(&analysis, &cfg);

        // Check that x and y are both Int at exit
        let exit_state = result
            .out_facts
            .values()
            .find(|s| s.get("x") == LatticeType::Concrete(Type::Int));
        assert!(exit_state.is_some());
    }

    #[test]
    fn test_parameter_type_propagation() {
        let func = make_function(
            "add",
            vec![
                HirParam::new("a".to_string(), Type::Int),
                HirParam::new("b".to_string(), Type::Int),
            ],
            vec![HirStmt::Return(Some(HirExpr::Binary {
                op: BinOp::Add,
                left: Box::new(HirExpr::Var("a".to_string())),
                right: Box::new(HirExpr::Var("b".to_string())),
            }))],
        );

        let mut param_types = HashMap::new();
        param_types.insert("a".to_string(), Type::Int);
        param_types.insert("b".to_string(), Type::Int);

        let cfg = CfgBuilder::new().build_function(&func);
        let analysis = TypePropagation::new(param_types);
        let result = FixpointSolver::solve(&analysis, &cfg);

        // Parameters should be Int throughout
        let entry_in = result.in_facts.get(&cfg.entry).unwrap();
        assert_eq!(entry_in.get("a"), LatticeType::Concrete(Type::Int));
        assert_eq!(entry_in.get("b"), LatticeType::Concrete(Type::Int));
    }

    #[test]
    fn test_if_branch_type_join() {
        let func = make_function(
            "test",
            vec![HirParam::new("cond".to_string(), Type::Bool)],
            vec![
                HirStmt::If {
                    condition: HirExpr::Var("cond".to_string()),
                    then_body: vec![HirStmt::Assign {
                        target: AssignTarget::Symbol("x".to_string()),
                        value: HirExpr::Literal(Literal::Int(1)),
                        type_annotation: None,
                    }],
                    else_body: Some(vec![HirStmt::Assign {
                        target: AssignTarget::Symbol("x".to_string()),
                        value: HirExpr::Literal(Literal::Int(2)),
                        type_annotation: None,
                    }]),
                },
                HirStmt::Return(Some(HirExpr::Var("x".to_string()))),
            ],
        );

        let mut param_types = HashMap::new();
        param_types.insert("cond".to_string(), Type::Bool);

        let cfg = CfgBuilder::new().build_function(&func);
        let analysis = TypePropagation::new(param_types);
        let result = FixpointSolver::solve(&analysis, &cfg);

        // After merge, x should still be Int (same type in both branches)
        let has_int_x = result
            .out_facts
            .values()
            .any(|s| s.get("x") == LatticeType::Concrete(Type::Int));
        assert!(has_int_x);
    }

    #[test]
    fn test_expr_type_inference() {
        let analysis = TypePropagation::new(HashMap::new());
        let state = TypeState::new();

        // Literal inference
        assert_eq!(
            analysis.infer_expr_type(&HirExpr::Literal(Literal::Int(42)), &state),
            Type::Int
        );
        assert_eq!(
            analysis.infer_expr_type(&HirExpr::Literal(Literal::Float(3.14)), &state),
            Type::Float
        );
        assert_eq!(
            analysis.infer_expr_type(&HirExpr::Literal(Literal::String("hello".to_string())), &state),
            Type::String
        );
        assert_eq!(
            analysis.infer_expr_type(&HirExpr::Literal(Literal::Bool(true)), &state),
            Type::Bool
        );
    }

    #[test]
    fn test_binary_op_type_inference() {
        let analysis = TypePropagation::new(HashMap::new());
        let mut state = TypeState::new();
        state.set("x".to_string(), LatticeType::Concrete(Type::Int));
        state.set("y".to_string(), LatticeType::Concrete(Type::Int));

        let add_expr = HirExpr::Binary {
            op: BinOp::Add,
            left: Box::new(HirExpr::Var("x".to_string())),
            right: Box::new(HirExpr::Var("y".to_string())),
        };
        assert_eq!(analysis.infer_expr_type(&add_expr, &state), Type::Int);

        let div_expr = HirExpr::Binary {
            op: BinOp::Div,
            left: Box::new(HirExpr::Var("x".to_string())),
            right: Box::new(HirExpr::Var("y".to_string())),
        };
        assert_eq!(analysis.infer_expr_type(&div_expr, &state), Type::Float);
    }

    #[test]
    fn test_list_type_inference() {
        let analysis = TypePropagation::new(HashMap::new());
        let state = TypeState::new();

        let list_expr = HirExpr::List(vec![
            HirExpr::Literal(Literal::Int(1)),
            HirExpr::Literal(Literal::Int(2)),
        ]);
        assert_eq!(
            analysis.infer_expr_type(&list_expr, &state),
            Type::List(Box::new(Type::Int))
        );
    }

    #[test]
    fn test_builtin_call_type_inference() {
        let analysis = TypePropagation::new(HashMap::new());
        let mut state = TypeState::new();
        state.set(
            "items".to_string(),
            LatticeType::Concrete(Type::List(Box::new(Type::Int))),
        );

        // len() returns int
        assert_eq!(
            analysis.infer_call_type("len", &[HirExpr::Var("items".to_string())], &state),
            Type::Int
        );

        // str() returns string
        assert_eq!(
            analysis.infer_call_type("str", &[HirExpr::Literal(Literal::Int(42))], &state),
            Type::String
        );
    }

    #[test]
    fn test_method_call_type_inference() {
        let analysis = TypePropagation::new(HashMap::new());
        let mut state = TypeState::new();
        state.set("s".to_string(), LatticeType::Concrete(Type::String));
        state.set(
            "items".to_string(),
            LatticeType::Concrete(Type::List(Box::new(Type::Int))),
        );

        // String.upper() returns String
        assert_eq!(
            analysis.infer_method_call_type(&HirExpr::Var("s".to_string()), "upper", &[], &state),
            Type::String
        );

        // String.split() returns List[String]
        assert_eq!(
            analysis.infer_method_call_type(&HirExpr::Var("s".to_string()), "split", &[], &state),
            Type::List(Box::new(Type::String))
        );

        // List.pop() returns element type
        assert_eq!(
            analysis.infer_method_call_type(&HirExpr::Var("items".to_string()), "pop", &[], &state),
            Type::Int
        );
    }

    #[test]
    fn test_fixpoint_convergence() {
        // Test that the solver converges in reasonable iterations
        let func = make_function(
            "loop_test",
            vec![HirParam::new("n".to_string(), Type::Int)],
            vec![
                HirStmt::Assign {
                    target: AssignTarget::Symbol("i".to_string()),
                    value: HirExpr::Literal(Literal::Int(0)),
                    type_annotation: None,
                },
                HirStmt::While {
                    condition: HirExpr::Binary {
                        op: BinOp::Lt,
                        left: Box::new(HirExpr::Var("i".to_string())),
                        right: Box::new(HirExpr::Var("n".to_string())),
                    },
                    body: vec![HirStmt::Assign {
                        target: AssignTarget::Symbol("i".to_string()),
                        value: HirExpr::Binary {
                            op: BinOp::Add,
                            left: Box::new(HirExpr::Var("i".to_string())),
                            right: Box::new(HirExpr::Literal(Literal::Int(1))),
                        },
                        type_annotation: None,
                    }],
                },
                HirStmt::Return(Some(HirExpr::Var("i".to_string()))),
            ],
        );

        let mut param_types = HashMap::new();
        param_types.insert("n".to_string(), Type::Int);

        let cfg = CfgBuilder::new().build_function(&func);
        let analysis = TypePropagation::new(param_types);
        let result = FixpointSolver::solve(&analysis, &cfg);

        // Should converge quickly (within ~10 iterations for this simple case)
        assert!(result.iterations < 50);

        // i should be Int after the loop
        let has_int_i = result
            .out_facts
            .values()
            .any(|s| s.get("i") == LatticeType::Concrete(Type::Int));
        assert!(has_int_i);
    }
}
