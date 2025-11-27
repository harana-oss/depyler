//! Control Flow Graph construction from HIR

use crate::hir::{AssignTarget, BinOp, HirExpr, HirFunction, HirStmt, Type};
use std::collections::HashMap;

/// Unique identifier for a basic block
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub usize);

impl BlockId {
    pub const ENTRY: BlockId = BlockId(0);
    pub const EXIT: BlockId = BlockId(usize::MAX);
}

/// Edge in the CFG connecting two blocks
#[derive(Debug, Clone)]
pub struct CfgEdge {
    pub from: BlockId,
    pub to: BlockId,
    pub condition: Option<EdgeCondition>,
}

/// Condition under which an edge is taken
#[derive(Debug, Clone)]
pub enum EdgeCondition {
    True,
    False,
    Unconditional,
}

/// How a basic block terminates
#[derive(Debug, Clone)]
pub enum Terminator {
    /// Unconditional jump to another block
    Goto(BlockId),
    /// Conditional branch
    Branch {
        condition: HirExpr,
        then_block: BlockId,
        else_block: BlockId,
    },
    /// Return from function
    Return(Option<HirExpr>),
    /// Loop back edge
    Loop {
        condition: HirExpr,
        body_block: BlockId,
        exit_block: BlockId,
    },
    /// Unreachable (after break/continue)
    Unreachable,
}

/// A statement in a basic block (simplified for dataflow)
#[derive(Debug, Clone)]
pub enum CfgStmt {
    /// Variable assignment
    Assign {
        target: String,
        value: HirExpr,
        type_annotation: Option<Type>,
    },
    /// Index assignment (a[i] = v)
    IndexAssign {
        base: String,
        index: HirExpr,
        value: HirExpr,
    },
    /// Expression statement
    Expr(HirExpr),
    /// Assert statement
    Assert { test: HirExpr, msg: Option<HirExpr> },
    /// Pass (no-op)
    Pass,
}

/// A basic block in the CFG
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub stmts: Vec<CfgStmt>,
    pub terminator: Option<Terminator>,
    pub predecessors: Vec<BlockId>,
    pub successors: Vec<BlockId>,
}

impl BasicBlock {
    pub fn new(id: BlockId) -> Self {
        Self {
            id,
            stmts: Vec::new(),
            terminator: None,
            predecessors: Vec::new(),
            successors: Vec::new(),
        }
    }
}

/// Control Flow Graph
#[derive(Debug)]
pub struct Cfg {
    pub blocks: HashMap<BlockId, BasicBlock>,
    pub entry: BlockId,
    pub exit: BlockId,
    next_block_id: usize,
}

impl Cfg {
    pub fn new() -> Self {
        let mut cfg = Self {
            blocks: HashMap::new(),
            entry: BlockId::ENTRY,
            exit: BlockId::EXIT,
            next_block_id: 0,
        };

        // Create entry block
        let entry = cfg.new_block();
        cfg.entry = entry;

        // Create exit block
        let exit = cfg.new_block();
        cfg.exit = exit;

        cfg
    }

    pub fn new_block(&mut self) -> BlockId {
        let id = BlockId(self.next_block_id);
        self.next_block_id += 1;
        self.blocks.insert(id, BasicBlock::new(id));
        id
    }

    pub fn add_edge(&mut self, from: BlockId, to: BlockId) {
        if let Some(block) = self.blocks.get_mut(&from) {
            if !block.successors.contains(&to) {
                block.successors.push(to);
            }
        }
        if let Some(block) = self.blocks.get_mut(&to) {
            if !block.predecessors.contains(&from) {
                block.predecessors.push(from);
            }
        }
    }

    pub fn set_terminator(&mut self, block: BlockId, terminator: Terminator) {
        if let Some(b) = self.blocks.get_mut(&block) {
            b.terminator = Some(terminator);
        }
    }

    pub fn add_stmt(&mut self, block: BlockId, stmt: CfgStmt) {
        if let Some(b) = self.blocks.get_mut(&block) {
            b.stmts.push(stmt);
        }
    }

    /// Get blocks in reverse postorder (useful for forward dataflow)
    pub fn reverse_postorder(&self) -> Vec<BlockId> {
        let mut visited = std::collections::HashSet::new();
        let mut postorder = Vec::new();
        self.dfs_postorder(self.entry, &mut visited, &mut postorder);
        postorder.reverse();
        postorder
    }

    fn dfs_postorder(
        &self,
        block: BlockId,
        visited: &mut std::collections::HashSet<BlockId>,
        postorder: &mut Vec<BlockId>,
    ) {
        if visited.contains(&block) {
            return;
        }
        visited.insert(block);

        if let Some(b) = self.blocks.get(&block) {
            for &succ in &b.successors {
                self.dfs_postorder(succ, visited, postorder);
            }
        }
        postorder.push(block);
    }

    /// Get blocks in postorder (useful for backward dataflow)
    pub fn postorder(&self) -> Vec<BlockId> {
        let mut visited = std::collections::HashSet::new();
        let mut result = Vec::new();
        self.dfs_postorder(self.entry, &mut visited, &mut result);
        result
    }
}

impl Default for Cfg {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for constructing CFG from HIR
pub struct CfgBuilder {
    cfg: Cfg,
    current_block: BlockId,
    loop_stack: Vec<LoopContext>,
}

struct LoopContext {
    continue_block: BlockId,
    break_block: BlockId,
}

impl CfgBuilder {
    pub fn new() -> Self {
        let cfg = Cfg::new();
        let entry = cfg.entry;
        Self {
            cfg,
            current_block: entry,
            loop_stack: Vec::new(),
        }
    }

    pub fn build_function(mut self, func: &HirFunction) -> Cfg {
        // Add parameter assignments as initial definitions
        for param in &func.params {
            self.cfg.add_stmt(
                self.current_block,
                CfgStmt::Assign {
                    target: param.name.clone(),
                    value: HirExpr::Var(param.name.clone()),
                    type_annotation: Some(param.ty.clone()),
                },
            );
        }

        // Process function body
        self.build_body(&func.body);

        // If no explicit return, add implicit return None
        if self
            .cfg
            .blocks
            .get(&self.current_block)
            .is_some_and(|b| b.terminator.is_none())
        {
            self.cfg.set_terminator(self.current_block, Terminator::Return(None));
            self.cfg.add_edge(self.current_block, self.cfg.exit);
        }

        self.cfg
    }

    fn build_body(&mut self, stmts: &[HirStmt]) {
        for stmt in stmts {
            self.build_stmt(stmt);
        }
    }

    fn build_stmt(&mut self, stmt: &HirStmt) {
        match stmt {
            HirStmt::Assign {
                target,
                value,
                type_annotation,
            } => {
                self.build_assign(target, value, type_annotation.clone());
            }
            HirStmt::Return(expr) => {
                self.cfg
                    .set_terminator(self.current_block, Terminator::Return(expr.clone()));
                self.cfg.add_edge(self.current_block, self.cfg.exit);
                // Create new unreachable block for any following statements
                self.current_block = self.cfg.new_block();
            }
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => {
                self.build_if(condition, then_body, else_body);
            }
            HirStmt::While { condition, body } => {
                self.build_while(condition, body);
            }
            HirStmt::For { target, iter, body } => {
                self.build_for(target, iter, body);
            }
            HirStmt::Expr(expr) => {
                self.cfg.add_stmt(self.current_block, CfgStmt::Expr(expr.clone()));
            }
            HirStmt::Break { .. } => {
                if let Some(ctx) = self.loop_stack.last() {
                    self.cfg
                        .set_terminator(self.current_block, Terminator::Goto(ctx.break_block));
                    self.cfg.add_edge(self.current_block, ctx.break_block);
                }
                self.current_block = self.cfg.new_block();
            }
            HirStmt::Continue { .. } => {
                if let Some(ctx) = self.loop_stack.last() {
                    self.cfg
                        .set_terminator(self.current_block, Terminator::Goto(ctx.continue_block));
                    self.cfg.add_edge(self.current_block, ctx.continue_block);
                }
                self.current_block = self.cfg.new_block();
            }
            HirStmt::Pass => {
                self.cfg.add_stmt(self.current_block, CfgStmt::Pass);
            }
            HirStmt::Assert { test, msg } => {
                self.cfg.add_stmt(
                    self.current_block,
                    CfgStmt::Assert {
                        test: test.clone(),
                        msg: msg.clone(),
                    },
                );
            }
            HirStmt::Raise { .. } => {
                // Raise terminates control flow similar to return
                self.cfg.set_terminator(self.current_block, Terminator::Unreachable);
                self.current_block = self.cfg.new_block();
            }
            HirStmt::Try {
                body,
                handlers,
                orelse,
                finalbody,
            } => {
                // Simplified: treat try block as sequential
                self.build_body(body);
                for handler in handlers {
                    self.build_body(&handler.body);
                }
                if let Some(else_stmts) = orelse {
                    self.build_body(else_stmts);
                }
                if let Some(finally_stmts) = finalbody {
                    self.build_body(finally_stmts);
                }
            }
            HirStmt::With {
                context: _,
                target: _,
                body,
            } => {
                self.build_body(body);
            }
            HirStmt::FunctionDef { .. } => {
                // Nested function definitions don't affect outer CFG
            }
        }
    }

    fn build_assign(&mut self, target: &AssignTarget, value: &HirExpr, type_annotation: Option<Type>) {
        match target {
            AssignTarget::Symbol(name) => {
                self.cfg.add_stmt(
                    self.current_block,
                    CfgStmt::Assign {
                        target: name.clone(),
                        value: value.clone(),
                        type_annotation,
                    },
                );
            }
            AssignTarget::Index { base, index } => {
                if let HirExpr::Var(name) = base.as_ref() {
                    self.cfg.add_stmt(
                        self.current_block,
                        CfgStmt::IndexAssign {
                            base: name.clone(),
                            index: *index.clone(),
                            value: value.clone(),
                        },
                    );
                }
            }
            AssignTarget::Tuple(targets) => {
                // Handle tuple unpacking
                for (i, sub_target) in targets.iter().enumerate() {
                    let sub_value = HirExpr::Index {
                        base: Box::new(value.clone()),
                        index: Box::new(HirExpr::Literal(crate::hir::Literal::Int(i as i64))),
                    };
                    self.build_assign(sub_target, &sub_value, None);
                }
            }
            AssignTarget::Attribute { value: _, attr: _ } => {
                // Attribute assignment - simplified handling
            }
        }
    }

    fn build_if(&mut self, condition: &HirExpr, then_body: &[HirStmt], else_body: &Option<Vec<HirStmt>>) {
        let then_block = self.cfg.new_block();
        let else_block = self.cfg.new_block();
        let merge_block = self.cfg.new_block();

        // Current block branches
        self.cfg.set_terminator(
            self.current_block,
            Terminator::Branch {
                condition: condition.clone(),
                then_block,
                else_block,
            },
        );
        self.cfg.add_edge(self.current_block, then_block);
        self.cfg.add_edge(self.current_block, else_block);

        // Build then branch
        self.current_block = then_block;
        self.build_body(then_body);
        if self
            .cfg
            .blocks
            .get(&self.current_block)
            .is_some_and(|b| b.terminator.is_none())
        {
            self.cfg
                .set_terminator(self.current_block, Terminator::Goto(merge_block));
            self.cfg.add_edge(self.current_block, merge_block);
        }

        // Build else branch
        self.current_block = else_block;
        if let Some(else_stmts) = else_body {
            self.build_body(else_stmts);
        }
        if self
            .cfg
            .blocks
            .get(&self.current_block)
            .is_some_and(|b| b.terminator.is_none())
        {
            self.cfg
                .set_terminator(self.current_block, Terminator::Goto(merge_block));
            self.cfg.add_edge(self.current_block, merge_block);
        }

        self.current_block = merge_block;
    }

    fn build_while(&mut self, condition: &HirExpr, body: &[HirStmt]) {
        let header_block = self.cfg.new_block();
        let body_block = self.cfg.new_block();
        let exit_block = self.cfg.new_block();

        // Jump to header
        self.cfg
            .set_terminator(self.current_block, Terminator::Goto(header_block));
        self.cfg.add_edge(self.current_block, header_block);

        // Header block with loop condition
        self.cfg.set_terminator(
            header_block,
            Terminator::Loop {
                condition: condition.clone(),
                body_block,
                exit_block,
            },
        );
        self.cfg.add_edge(header_block, body_block);
        self.cfg.add_edge(header_block, exit_block);

        // Build loop body
        self.loop_stack.push(LoopContext {
            continue_block: header_block,
            break_block: exit_block,
        });
        self.current_block = body_block;
        self.build_body(body);
        self.loop_stack.pop();

        // Back edge to header
        if self
            .cfg
            .blocks
            .get(&self.current_block)
            .is_some_and(|b| b.terminator.is_none())
        {
            self.cfg
                .set_terminator(self.current_block, Terminator::Goto(header_block));
            self.cfg.add_edge(self.current_block, header_block);
        }

        self.current_block = exit_block;
    }

    fn build_for(&mut self, target: &AssignTarget, iter: &HirExpr, body: &[HirStmt]) {
        let header_block = self.cfg.new_block();
        let body_block = self.cfg.new_block();
        let exit_block = self.cfg.new_block();

        // Jump to header
        self.cfg
            .set_terminator(self.current_block, Terminator::Goto(header_block));
        self.cfg.add_edge(self.current_block, header_block);

        // Header - assign iterator element to target (simplified)
        if let AssignTarget::Symbol(name) = target {
            // Create synthetic assignment for loop variable
            let elem_expr = HirExpr::Call {
                func: "__iter_next__".to_string(),
                args: vec![iter.clone()],
                kwargs: vec![],
                type_params: vec![],
            };
            self.cfg.add_stmt(
                header_block,
                CfgStmt::Assign {
                    target: name.clone(),
                    value: elem_expr.clone(),
                    type_annotation: None,
                },
            );

            // Condition: has more elements
            let condition = HirExpr::Binary {
                op: BinOp::NotEq,
                left: Box::new(elem_expr),
                right: Box::new(HirExpr::Literal(crate::hir::Literal::None)),
            };
            self.cfg.set_terminator(
                header_block,
                Terminator::Loop {
                    condition,
                    body_block,
                    exit_block,
                },
            );
        } else {
            // For tuple unpacking, use simplified condition
            let condition = HirExpr::Literal(crate::hir::Literal::Bool(true));
            self.cfg.set_terminator(
                header_block,
                Terminator::Loop {
                    condition,
                    body_block,
                    exit_block,
                },
            );
        }

        self.cfg.add_edge(header_block, body_block);
        self.cfg.add_edge(header_block, exit_block);

        // Build loop body
        self.loop_stack.push(LoopContext {
            continue_block: header_block,
            break_block: exit_block,
        });
        self.current_block = body_block;
        self.build_body(body);
        self.loop_stack.pop();

        // Back edge
        if self
            .cfg
            .blocks
            .get(&self.current_block)
            .is_some_and(|b| b.terminator.is_none())
        {
            self.cfg
                .set_terminator(self.current_block, Terminator::Goto(header_block));
            self.cfg.add_edge(self.current_block, header_block);
        }

        self.current_block = exit_block;
    }
}

impl Default for CfgBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::{FunctionProperties, HirParam, Literal};
    use depyler_annotations::TranspilationAnnotations;
    use smallvec::smallvec;

    fn make_function(name: &str, params: Vec<HirParam>, body: Vec<HirStmt>) -> HirFunction {
        HirFunction {
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
    fn test_cfg_simple_function() {
        let func = make_function(
            "simple",
            vec![HirParam::new("x".to_string(), Type::Int)],
            vec![HirStmt::Return(Some(HirExpr::Var("x".to_string())))],
        );

        let cfg = CfgBuilder::new().build_function(&func);

        assert!(cfg.blocks.len() >= 2);
        assert!(cfg.blocks.contains_key(&cfg.entry));
        assert!(cfg.blocks.contains_key(&cfg.exit));
    }

    #[test]
    fn test_cfg_if_statement() {
        let func = make_function(
            "with_if",
            vec![HirParam::new("x".to_string(), Type::Int)],
            vec![HirStmt::If {
                condition: HirExpr::Binary {
                    op: BinOp::Gt,
                    left: Box::new(HirExpr::Var("x".to_string())),
                    right: Box::new(HirExpr::Literal(Literal::Int(0))),
                },
                then_body: vec![HirStmt::Return(Some(HirExpr::Literal(Literal::Int(1))))],
                else_body: Some(vec![HirStmt::Return(Some(HirExpr::Literal(Literal::Int(0))))]),
            }],
        );

        let cfg = CfgBuilder::new().build_function(&func);

        // Should have entry, then block, else block, merge block, exit
        assert!(cfg.blocks.len() >= 4);

        // Entry block should have branch terminator
        let entry_block = cfg.blocks.get(&cfg.entry).unwrap();
        assert!(matches!(entry_block.terminator, Some(Terminator::Branch { .. })));
    }

    #[test]
    fn test_cfg_while_loop() {
        let func = make_function(
            "with_while",
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

        let cfg = CfgBuilder::new().build_function(&func);

        // Should have back edge for the loop
        let has_loop_terminator = cfg
            .blocks
            .values()
            .any(|b| matches!(b.terminator, Some(Terminator::Loop { .. })));
        assert!(has_loop_terminator);
    }

    #[test]
    fn test_reverse_postorder() {
        let func = make_function(
            "simple",
            vec![],
            vec![HirStmt::Return(Some(HirExpr::Literal(Literal::Int(42))))],
        );

        let cfg = CfgBuilder::new().build_function(&func);
        let rpo = cfg.reverse_postorder();

        // Entry should come first in RPO
        assert_eq!(rpo[0], cfg.entry);
    }

    #[test]
    fn test_cfg_for_loop() {
        let func = make_function(
            "with_for",
            vec![HirParam::new("items".to_string(), Type::List(Box::new(Type::Int)))],
            vec![
                HirStmt::Assign {
                    target: AssignTarget::Symbol("total".to_string()),
                    value: HirExpr::Literal(Literal::Int(0)),
                    type_annotation: None,
                },
                HirStmt::For {
                    target: AssignTarget::Symbol("item".to_string()),
                    iter: HirExpr::Var("items".to_string()),
                    body: vec![HirStmt::Assign {
                        target: AssignTarget::Symbol("total".to_string()),
                        value: HirExpr::Binary {
                            op: BinOp::Add,
                            left: Box::new(HirExpr::Var("total".to_string())),
                            right: Box::new(HirExpr::Var("item".to_string())),
                        },
                        type_annotation: None,
                    }],
                },
                HirStmt::Return(Some(HirExpr::Var("total".to_string()))),
            ],
        );

        let cfg = CfgBuilder::new().build_function(&func);

        // Should have loop structure
        let has_loop = cfg
            .blocks
            .values()
            .any(|b| matches!(b.terminator, Some(Terminator::Loop { .. })));
        assert!(has_loop);
    }

    #[test]
    fn test_cfg_break_continue() {
        let func = make_function(
            "with_break",
            vec![],
            vec![HirStmt::While {
                condition: HirExpr::Literal(Literal::Bool(true)),
                body: vec![HirStmt::If {
                    condition: HirExpr::Literal(Literal::Bool(true)),
                    then_body: vec![HirStmt::Break { label: None }],
                    else_body: Some(vec![HirStmt::Continue { label: None }]),
                }],
            }],
        );

        let cfg = CfgBuilder::new().build_function(&func);

        // Should have multiple blocks due to break/continue
        assert!(cfg.blocks.len() >= 5);
    }
}
