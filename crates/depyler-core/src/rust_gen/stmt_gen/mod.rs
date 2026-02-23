use crate::hir::*;
use crate::rust_gen::context::{CodeGenContext, RustCodeGen};
use anyhow::Result;

pub(crate) mod helpers;
pub(crate) mod type_helpers;
pub(crate) mod assign;
pub(crate) mod control_flow;
pub(crate) mod return_stmt;
pub(crate) mod exception;
pub(crate) mod context_mgr;
pub(crate) mod pattern_match;
pub(crate) mod functions;
pub(crate) mod argparse;
pub(crate) mod simple;

pub(crate) use helpers::*;
pub(crate) use type_helpers::*;
pub(crate) use assign::*;
pub(crate) use control_flow::*;
pub(crate) use return_stmt::*;
pub(crate) use exception::*;
pub(crate) use context_mgr::*;
pub(crate) use pattern_match::*;
pub(crate) use functions::*;
pub(crate) use argparse::*;
pub(crate) use simple::*;

impl RustCodeGen for HirStmt {
    fn to_rust_tokens(&self, ctx: &mut CodeGenContext) -> Result<proc_macro2::TokenStream> {
        ctx.clone_already_applied = false;
        match self {
            HirStmt::Assign {
                target,
                value,
                type_annotation,
            } => assign::codegen_assign_stmt(target, value, type_annotation, ctx),
            HirStmt::Return(expr) => return_stmt::codegen_return_stmt(expr, ctx),
            HirStmt::If {
                condition,
                then_body,
                else_body,
            } => control_flow::codegen_if_stmt(condition, then_body, else_body, ctx),
            HirStmt::While { condition, body } => control_flow::codegen_while_stmt(condition, body, ctx),
            HirStmt::For { target, iter, body } => control_flow::codegen_for_stmt(target, iter, body, ctx),
            HirStmt::Expr(expr) => simple::codegen_expr_stmt(expr, ctx),
            HirStmt::Raise {
                exception,
                cause: _,
            } => exception::codegen_raise_stmt(exception, ctx),
            HirStmt::Break { label } => control_flow::codegen_break_stmt(label),
            HirStmt::Continue { label } => control_flow::codegen_continue_stmt(label),
            HirStmt::With {
                context,
                target,
                body,
            } => context_mgr::codegen_with_stmt(context, target, body, ctx),
            HirStmt::Try {
                body,
                handlers,
                orelse: _,
                finalbody,
            } => exception::codegen_try_stmt(body, handlers, finalbody, ctx),
            HirStmt::Assert { test, msg } => simple::codegen_assert_stmt(test, msg, ctx),
            HirStmt::Pass => simple::codegen_pass_stmt(),
            HirStmt::FunctionDef {
                name,
                params,
                ret_type,
                body,
                docstring: _,
            } => functions::codegen_nested_function_def(name, params, ret_type, body, ctx),
            HirStmt::Global { names } => functions::codegen_global_stmt(names),
            HirStmt::Nonlocal { names } => functions::codegen_nonlocal_stmt(names),
            HirStmt::AsyncFor { target, iter, body } => {
                simple::codegen_async_for_stmt(target, iter, body, ctx)
            }
            HirStmt::AsyncWith {
                context,
                target,
                body,
            } => context_mgr::codegen_async_with_stmt(context, target, body, ctx),
            HirStmt::Delete { targets } => simple::codegen_delete_stmt(targets, ctx),
            HirStmt::Import { .. } => simple::codegen_import_stmt(),
            HirStmt::ImportFrom { .. } => simple::codegen_import_from_stmt(),
            HirStmt::AsyncFunctionDef {
                name,
                params,
                ret_type,
                body,
                docstring: _,
            } => functions::codegen_async_nested_function_def(name, params, ret_type, body, ctx),
            HirStmt::Match { subject, cases } => pattern_match::codegen_match_stmt(subject, cases, ctx),
        }
    }
}
