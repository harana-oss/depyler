//! Individual optimization pass families.
//!
//! Each submodule adds a group of `impl Optimizer` methods:
//! - [`constant_propagation`] – `propagate_constants_*`
//! - [`dead_code`] – `eliminate_dead_code_*`
//! - [`cse`] – `eliminate_common_subexpressions_*`

pub mod constant_propagation;
pub mod cse;
pub mod dead_code;
