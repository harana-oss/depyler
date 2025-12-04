mod test_helpers;

use test_helpers::transpile_and_check;

#[test]
fn test_string_variable_in_tuple() {
    let python = r#"
def check_location(location: str) -> bool:
    return location in ("Home", "Away")
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains(r#"["Home".to_string(), "Away".to_string()].contains(&location)"#));
}

#[test]
fn test_string_literal_in_tuple() {
    let python = r#"
def check_location() -> bool:
    return "Home" in ("Home", "Away")
"#;

    let rust = transpile_and_check(python, &[]);
    assert!(rust.contains(r#"["Home".to_string(), "Away".to_string()].contains"#));
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
    assert!(rust.contains(r#"["Home".to_string(), "Away".to_string()].contains"#));
    assert!(rust.contains("state.item"));
}
