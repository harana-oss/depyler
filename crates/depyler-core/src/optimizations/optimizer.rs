//! Optimization passes for generated Rust code.
//!
//! The three main pass families live in [`passes`] submodules and extend
//! [`Optimizer`] through separate `impl` blocks:
//! - constant propagation → [`crate::optimizations::passes::constant_propagation`]
//! - dead-code elimination → [`crate::optimizations::passes::dead_code`]
//! - common subexpression elimination → [`crate::optimizations::passes::cse`]

use crate::annotations::{OptimizationLevel, PerformanceHint};
use crate::hir::{BinOp, HirExpr, HirFunction, HirModule, HirStmt};

pub struct Optimizer {
    pub(crate) config: OptimizerConfig,
}

#[derive(Debug, Clone)]
pub struct OptimizerConfig {
    pub eliminate_dead_code: bool,
    pub propagate_constants: bool,
    pub eliminate_common_subexpressions: bool,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            eliminate_dead_code: false,
            propagate_constants: true,
            eliminate_common_subexpressions: true,
        }
    }
}

impl Optimizer {
    pub fn new(config: OptimizerConfig) -> Self {
        Self { config }
    }

    pub fn optimize_program(&mut self, mut program: HirModule) -> HirModule {
        if self.config.propagate_constants {
            program = self.propagate_constants_program(program);
        }

        if self.config.eliminate_dead_code {
            program = self.eliminate_dead_code_program(program);
        }

        if self.config.eliminate_common_subexpressions {
            program = self.eliminate_common_subexpressions_program(program);
        }

        program
    }
}

pub struct PerformanceOptimizer {
    optimizations_applied: Vec<String>,
}

impl Default for PerformanceOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl PerformanceOptimizer {
    pub fn new() -> Self {
        Self {
            optimizations_applied: Vec::new(),
        }
    }

    /// Optimize a function based on its annotations
    pub fn optimize_function(&mut self, func: &mut HirFunction) {
        match func.annotations.optimization_level {
            OptimizationLevel::Conservative => {
                self.apply_conservative_optimizations(func);
            }
            OptimizationLevel::Standard => {
                self.apply_standard_optimizations(func);
            }
            OptimizationLevel::Aggressive => {
                self.apply_aggressive_optimizations(func);
            }
        }

        let hints = func.annotations.performance_hints.clone();
        for hint in &hints {
            self.apply_performance_hint(func, hint);
        }
    }

    fn apply_conservative_optimizations(&mut self, func: &mut HirFunction) {
        self.constant_folding(&mut func.body);
        self.dead_code_elimination(&mut func.body);
    }

    fn apply_standard_optimizations(&mut self, func: &mut HirFunction) {
        self.apply_conservative_optimizations(func);
        self.common_subexpression_elimination(&mut func.body);
        self.strength_reduction(&mut func.body);
    }

    fn apply_aggressive_optimizations(&mut self, func: &mut HirFunction) {
        self.apply_standard_optimizations(func);
        self.loop_unrolling(&mut func.body, 4);
        self.inline_small_functions(&mut func.body);

        if func.annotations.bounds_checking == crate::annotations::BoundsChecking::Disabled {
            self.remove_bounds_checks(&mut func.body);
        }
    }

    fn apply_performance_hint(&mut self, func: &mut HirFunction, hint: &PerformanceHint) {
        match hint {
            PerformanceHint::Vectorize => {
                self.vectorize_loops(&mut func.body);
            }
            PerformanceHint::UnrollLoops(factor) => {
                self.loop_unrolling(&mut func.body, *factor as usize);
            }
            PerformanceHint::OptimizeForLatency => {
                self.optimize_for_latency(&mut func.body);
            }
            PerformanceHint::OptimizeForThroughput => {
                self.optimize_for_throughput(&mut func.body);
            }
            PerformanceHint::PerformanceCritical => {
                self.inline_small_functions(&mut func.body);
                self.vectorize_loops(&mut func.body);
            }
        }
    }

    fn constant_folding(&mut self, stmts: &mut Vec<HirStmt>) {
        for stmt in stmts {
            self.fold_constants_in_stmt(stmt);
        }
        self.optimizations_applied
            .push("constant_folding".to_string());
    }

    fn fold_constants_in_stmt(&mut self, stmt: &mut HirStmt) {
        match stmt {
            HirStmt::Assign { value, .. } => {
                self.fold_constants_expr(value);
            }
            HirStmt::Return(Some(expr)) => {
                self.fold_constants_expr(expr);
            }
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.fold_constants_in_if(condition, then_body, else_body);
            }
            HirStmt::While { condition, body } => {
                self.fold_constants_in_while(condition, body);
            }
            HirStmt::For { body, .. } => {
                self.constant_folding(body);
            }
            _ => {}
        }
    }

    fn fold_constants_in_if(
        &mut self,
        condition: &mut HirExpr,
        then_body: &mut Vec<HirStmt>,
        else_body: &mut Option<Vec<HirStmt>>,
    ) {
        self.fold_constants_expr(condition);
        self.constant_folding(then_body);
        if let Some(else_stmts) = else_body {
            self.constant_folding(else_stmts);
        }
    }

    fn fold_constants_in_while(&mut self, condition: &mut HirExpr, body: &mut Vec<HirStmt>) {
        self.fold_constants_expr(condition);
        self.constant_folding(body);
    }

    fn fold_constants_expr(&self, expr: &mut HirExpr) {
        if let HirExpr::Binary { op, left, right } = expr {
            self.fold_constants_expr(left);
            self.fold_constants_expr(right);

            if let (HirExpr::Literal(left_lit), HirExpr::Literal(right_lit)) =
                (left.as_ref(), right.as_ref())
            {
                if let Some(folded) = self.evaluate_binary_op(*op, left_lit, right_lit) {
                    *expr = HirExpr::Literal(folded);
                }
            }
        }
    }

    fn evaluate_binary_op(
        &self,
        op: BinOp,
        left: &crate::hir::Literal,
        right: &crate::hir::Literal,
    ) -> Option<crate::hir::Literal> {
        use crate::hir::Literal;

        match (op, left, right) {
            (BinOp::Add, Literal::Int(a), Literal::Int(b)) => Some(Literal::Int(a + b)),
            (BinOp::Sub, Literal::Int(a), Literal::Int(b)) => Some(Literal::Int(a - b)),
            (BinOp::Mul, Literal::Int(a), Literal::Int(b)) => Some(Literal::Int(a * b)),
            (BinOp::Add, Literal::Float(a), Literal::Float(b)) => Some(Literal::Float(a + b)),
            (BinOp::Sub, Literal::Float(a), Literal::Float(b)) => Some(Literal::Float(a - b)),
            (BinOp::Mul, Literal::Float(a), Literal::Float(b)) => Some(Literal::Float(a * b)),
            _ => None,
        }
    }

    fn dead_code_elimination(&mut self, stmts: &mut Vec<HirStmt>) {
        let mut found_return = false;
        stmts.retain(|stmt| {
            if found_return {
                false
            } else {
                if matches!(stmt, HirStmt::Return(_)) {
                    found_return = true;
                }
                true
            }
        });
        self.optimizations_applied
            .push("dead_code_elimination".to_string());
    }

    fn common_subexpression_elimination(&mut self, _stmts: &mut [HirStmt]) {
        self.optimizations_applied
            .push("common_subexpression_elimination".to_string());
    }

    fn strength_reduction(&mut self, stmts: &mut [HirStmt]) {
        for stmt in stmts {
            match stmt {
                HirStmt::Assign { value, .. } => {
                    self.reduce_strength_expr(value);
                }
                HirStmt::Return(Some(expr)) => {
                    self.reduce_strength_expr(expr);
                }
                _ => {}
            }
        }
        self.optimizations_applied
            .push("strength_reduction".to_string());
    }

    fn reduce_strength_expr(&self, expr: &mut HirExpr) {
        match expr {
            HirExpr::Binary {
                op: BinOp::Mul,
                left: _,
                right,
            } => {
                if let HirExpr::Literal(crate::hir::Literal::Int(_n)) = right.as_ref() {
                    // Strength reduction disabled for semantic correctness
                }
            }
            HirExpr::Binary {
                op: BinOp::Div,
                left: _,
                right,
            } => {
                if let HirExpr::Literal(crate::hir::Literal::Int(_n)) = right.as_ref() {
                    // Strength reduction disabled for semantic correctness
                }
            }
            _ => {}
        }
    }

    fn loop_unrolling(&mut self, stmts: &mut Vec<HirStmt>, factor: usize) {
        for stmt in stmts {
            if let HirStmt::For { body, .. } = stmt {
                let original_body = body.clone();
                for _ in 1..factor {
                    body.extend(original_body.clone());
                }
            }
        }
        self.optimizations_applied
            .push(format!("loop_unrolling_{factor}"));
    }

    fn vectorize_loops(&mut self, _stmts: &mut [HirStmt]) {
        self.optimizations_applied
            .push("vectorize_loops".to_string());
    }

    fn inline_small_functions(&mut self, _stmts: &mut [HirStmt]) {
        self.optimizations_applied
            .push("inline_small_functions".to_string());
    }

    fn remove_bounds_checks(&mut self, _stmts: &mut [HirStmt]) {
        self.optimizations_applied
            .push("remove_bounds_checks".to_string());
    }

    fn optimize_for_latency(&mut self, _stmts: &mut [HirStmt]) {
        self.optimizations_applied
            .push("optimize_for_latency".to_string());
    }

    fn optimize_for_throughput(&mut self, _stmts: &mut [HirStmt]) {
        self.optimizations_applied
            .push("optimize_for_throughput".to_string());
    }

    pub fn get_applied_optimizations(&self) -> &[String] {
        &self.optimizations_applied
    }
}

/// Apply optimizations to a module based on annotations
pub fn optimize_module(module: &mut crate::hir::HirModule) -> Vec<String> {
    let mut all_optimizations = Vec::new();

    for func in &mut module.functions {
        let mut optimizer = PerformanceOptimizer::new();
        optimizer.optimize_function(func);
        all_optimizations.extend(optimizer.get_applied_optimizations().to_vec());
    }

    all_optimizations
}
