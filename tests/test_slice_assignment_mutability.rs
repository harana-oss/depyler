//! Tests for slice assignment mutability detection and code generation
//!
//! These tests verify that:
//! 1. Slice assignment (e.g., `state.items[:] = value`) is detected as a mutation
//! 2. Parameters with slice assignment get `&mut` in generated code
//! 3. Generated code for slice assignment uses `.clear()` and `.extend()` without `.clone()`

#![allow(non_snake_case)]

use crate::test_helpers::transpile_and_check;

// ============================================================================
// Slice Assignment Mutation Detection
// ============================================================================

#[test]
fn test_slice_assignment_requires_mut_param() {
    // Slice assignment to a field should require &mut parameter
    let python = r#"
from dataclasses import dataclass

@dataclass
class Player:
    name: str
    score: int

@dataclass
class State:
    all_players: list[Player]

def refresh_players(state: State) -> None:
    combined = []
    state.all_players[:] = combined
"#;
    let rust_code = transpile_and_check(python, &[]);

    // Should generate &mut State parameter
    assert!(
        rust_code.contains("state: &mut State"),
        "Slice assignment should require &mut State parameter: {}",
        rust_code
    );
}

#[test]
fn test_slice_assignment_generates_clear_extend() {
    // Slice assignment should generate clear() and extend() without clone()
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    items: list[int]

def update_items(state: State) -> None:
    new_items = [1, 2, 3]
    state.items[:] = new_items
"#;
    let rust_code = transpile_and_check(python, &[]);

    // Should generate clear() call
    assert!(
        rust_code.contains(".clear()"),
        "Slice assignment should generate .clear(): {}",
        rust_code
    );

    // Should generate extend() call
    assert!(
        rust_code.contains(".extend("),
        "Slice assignment should generate .extend(): {}",
        rust_code
    );

    // Should NOT clone the field before clear/extend
    // Bad pattern: state.items.clone().clear()
    assert!(
        !rust_code.contains(".clone().clear()"),
        "Should NOT clone field before clear(): {}",
        rust_code
    );
    assert!(
        !rust_code.contains(".clone().extend("),
        "Should NOT clone field before extend(): {}",
        rust_code
    );
}

#[test]
fn test_slice_assignment_nested_attribute() {
    // Nested attribute slice assignment should also require &mut
    let python = r#"
from dataclasses import dataclass

@dataclass
class Inner:
    values: list[int]

@dataclass
class State:
    data: Inner

def update_nested(state: State) -> None:
    state.data.values[:] = [1, 2, 3]
"#;
    let rust_code = transpile_and_check(python, &[]);

    // Should generate &mut State parameter
    assert!(
        rust_code.contains("state: &mut State"),
        "Nested slice assignment should require &mut State parameter: {}",
        rust_code
    );
}

#[test]
fn test_slice_assignment_caller_passes_mut_ref() {
    // When calling a function that uses slice assignment, caller should pass &mut
    let python = r#"
from dataclasses import dataclass

@dataclass
class State:
    items: list[int]

def refresh_items(state: State) -> None:
    new_items = [1, 2, 3]
    state.items[:] = new_items

def process(state: State) -> None:
    refresh_items(state)
"#;
    let rust_code = transpile_and_check(python, &[]);

    // refresh_items should have &mut State
    assert!(
        rust_code.contains("fn refresh_items(state: &mut State)"),
        "refresh_items should have &mut State parameter: {}",
        rust_code
    );

    // process should pass &mut state (or state if it already has &mut)
    // The call should work without type mismatch
    assert!(
        rust_code.contains("refresh_items"),
        "Should contain call to refresh_items: {}",
        rust_code
    );
}

#[test]
fn test_slice_assignment_with_loop() {
    // Real-world pattern: combining lists in a loop, then slice-assigning
    let python = r#"
from dataclasses import dataclass

@dataclass
class Player:
    name: str

@dataclass  
class State:
    home_players: list[Player]
    away_players: list[Player]
    all_players: list[Player]

def get_team_players(state: State, team: str) -> list[Player]:
    if team == "Home":
        return state.home_players
    else:
        return state.away_players

def refresh_all_players(state: State) -> None:
    combined = []
    for team in ["Home", "Away"]:
        combined.extend(get_team_players(state, team))
    state.all_players[:] = combined
"#;
    let rust_code = transpile_and_check(python, &[]);

    // refresh_all_players should have &mut State
    assert!(
        rust_code.contains("fn refresh_all_players") && rust_code.contains("&mut State"),
        "refresh_all_players should have &mut State due to slice assignment: {}",
        rust_code
    );

    // Should generate correct slice assignment pattern
    assert!(
        rust_code.contains("all_players.clear()") || rust_code.contains("all_players . clear()"),
        "Should generate all_players.clear(): {}",
        rust_code
    );
}

#[test]
fn test_slice_assignment_simple_variable() {
    // Slice assignment to a simple variable (not a field)
    let python = r#"
def update_list() -> list[int]:
    items = [1, 2, 3]
    new_items = [4, 5, 6]
    items[:] = new_items
    return items
"#;
    let rust_code = transpile_and_check(python, &[]);

    // Should still generate clear() and extend()
    assert!(
        rust_code.contains(".clear()"),
        "Variable slice assignment should generate .clear(): {}",
        rust_code
    );
    assert!(
        rust_code.contains(".extend("),
        "Variable slice assignment should generate .extend(): {}",
        rust_code
    );
}
