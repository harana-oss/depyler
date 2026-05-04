//! Unreachable code elimination pass for [`PerformanceOptimizer`].
//!
//! Removes statements that are unreachable after a `return`.

use super::super::optimizer::PerformanceOptimizer;
use crate::hir::HirStmt;

impl PerformanceOptimizer {
    pub(crate) fn dead_code_elimination(&mut self, stmts: &mut Vec<HirStmt>) {
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
}
