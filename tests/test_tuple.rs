use crate::test_helpers::transpile_and_check;

#[test]
fn test_string_variable_in_tuple() {
    let python = r#"
def check_location(location: str) -> bool:
    return location in ("Home", "Away")
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains(r#"["Home", "Away"].contains(&location.as_str())"#));
}

#[test]
fn test_string_literal_in_tuple() {
    let python = r#"
def check_location() -> bool:
    return "Home" in ("Home", "Away")
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains(r#"["Home", "Away"].contains("Home")"#));
}

#[test]
fn test_string_struct_in_tuple() {
    let python = r#"

@dataclass
class State:
  item: str

def check_location(state: State) -> bool:
    return state.item in ("Home", "Away")
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains(r#"["Home", "Away"].contains(&state.item.as_str())"#));
}
