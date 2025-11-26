//! True dataflow analysis for type inference
//!
//! This module implements a proper dataflow analysis framework with:
//! - Control Flow Graph (CFG) construction from HIR
//! - Type lattice with join/meet operations
//! - Forward dataflow analysis for type propagation
//! - Worklist-based fixpoint solver
//! - Modular mutation tracking for container type inference

mod cfg;
mod lattice;
pub mod mutations;
mod solver;
mod type_inference;

pub use cfg::{BasicBlock, BlockId, Cfg, CfgBuilder, CfgEdge, Terminator};
pub use lattice::{TypeLattice, TypeState};
pub use mutations::MutationRegistry;
pub use solver::{DataflowAnalysis, DataflowDirection, FixpointSolver};
pub use type_inference::{DataflowTypeInferencer, InferredTypes};
