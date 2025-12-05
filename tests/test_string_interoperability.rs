use crate::test_helpers::{transpile, transpile_and_check};

#[test]
fn test_str_param_becomes_owned_string() {
    transpile_and_check(
        r#"
def greet(name: str) -> str:
    return name
"#,
        &["fn greet(name: String) -> String"],
    );
}

#[test]
fn test_string_literal_to_string_conversion() {
    transpile_and_check(
        r#"
def get_greeting() -> str:
    return "hello"
"#,
        &["\"hello\".to_string()"],
    );
}

#[test]
fn test_string_field_mutation_requires_to_string() {
    transpile_and_check(
        r#"
@dataclass
class State:
    value: str

def set_value(state: State) -> None:
    state.value = "updated"
"#,
        &["state.value = \"updated\".to_string()"],
    );
}

#[test]
fn test_string_clone_on_field_access() {
    transpile_and_check(
        r#"
@dataclass
class Container:
    text: str

def get_text(c: Container) -> str:
    return c.text
"#,
        &["c.text.clone()"],
    );
}

#[test]
fn test_string_clone_when_reused() {
    let rust_code = transpile(
        r#"
def take_string(s: str) -> str:
    return s + "!"

def use_twice(s: str) -> str:
    first = take_string(s)
    second = take_string(s)
    return first + second
"#,
    );
    assert!(
        rust_code.contains("take_string(s.clone())")
            || rust_code.contains("take_string(s.to_string())")
            || rust_code.contains("s: String"),
        "\n{rust_code}"
    );
}

#[test]
fn test_starts_with_takes_str_ref() {
    transpile_and_check(
        r#"
def check_prefix(s: str, prefix: str) -> bool:
    return s.startswith(prefix)
"#,
        &["s.starts_with(&prefix)"],
    );
}

#[test]
fn test_ends_with_takes_str_ref() {
    transpile_and_check(
        r#"
def check_suffix(s: str, suffix: str) -> bool:
    return s.endswith(suffix)
"#,
        &["s.ends_with(&suffix)"],
    );
}

#[test]
fn test_contains_takes_str_ref() {
    transpile_and_check(
        r#"
def has_substring(s: str, sub: str) -> bool:
    return sub in s
"#,
        &["s.contains(&sub)"],
    );
}

#[test]
fn test_join_takes_str_ref() {
    transpile_and_check(
        r#"
def join_parts(parts: list[str], sep: str) -> str:
    return sep.join(parts)
"#,
        &["parts.join(&sep)"],
    );
}

#[test]
fn test_split_returns_vec_string() {
    transpile_and_check(
        r#"
def split_csv(s: str) -> list[str]:
    return s.split(",")
"#,
        &["-> Vec<String>", ".map(|s| s.to_string()).collect"],
    );
}

#[test]
fn test_string_method_chain_ownership() {
    transpile_and_check(
        r#"
def clean(s: str) -> str:
    return s.strip()
"#,
        &["s.trim()"],
    );
}

#[test]
fn test_case_methods_return_string() {
    transpile_and_check(
        r#"
def to_lower(s: str) -> str:
    return s.lower()

def to_upper(s: str) -> str:
    return s.upper()
"#,
        &["s.to_lowercase()", "s.to_uppercase()"],
    );
}

#[test]
fn test_replace_with_str_literals() {
    transpile_and_check(
        r#"
def replace_spaces(s: str) -> str:
    return s.replace(" ", "_")
"#,
        &["s.replace(\" \", \"_\")"],
    );
}

#[test]
fn test_fstring_format_macro() {
    transpile_and_check(
        r#"
def format_name(name: str) -> str:
    return f"Hello, {name}!"
"#,
        &["format!(\"Hello, {}!\", name)"],
    );
}

#[test]
fn test_string_concatenation() {
    let rust_code = transpile(
        r#"
def concat(a: str, b: str) -> str:
    return a + b
"#,
    );
    assert!(
        rust_code.contains("a + &b") || rust_code.contains("format!"),
        "\n{rust_code}"
    );
}

#[test]
fn test_string_concatenation_with_field() {
    let rust_code = transpile(
        r#"
@dataclass
class State:
    a: str
    b: str

def concat_fields(state: State) -> None:
    state.b = state.a + "suffix"
"#,
    );
    assert!(
        rust_code.contains("format!") && rust_code.contains("state.a"),
        "\n{rust_code}"
    );
}

#[test]
fn test_optional_string() {
    transpile_and_check(
        r#"
from typing import Optional

def unwrap_or_default(val: Optional[str], default: str) -> str:
    if val is None:
        return default
    return val
"#,
        &["Option<String>", "-> String"],
    );
}

#[test]
fn test_string_repeat() {
    transpile_and_check(
        r#"
def repeat_string(s: str, n: int) -> str:
    return s * n
"#,
        &["s.repeat(n as usize)"],
    );
}

#[test]
fn test_int_to_string_conversion() {
    transpile_and_check(
        r#"
def int_to_string(n: int) -> str:
    return str(n)
"#,
        &["n.to_string()"],
    );
}

#[test]
fn test_string_to_int_parse() {
    transpile_and_check(
        r#"
def parse_int(s: str) -> int:
    return int(s)
"#,
        &["s.parse::<i32>().unwrap()"],
    );
}
#[test]
fn test_method_on_string_literal() {
    transpile_and_check(
        r#"
def get_upper() -> str:
    return "hello".upper()
"#,
        &["\"hello\".to_uppercase()"],
    );
}

#[test]
fn test_empty_string_literal() {
    transpile_and_check(
        r#"
def find_first(items: list[str], target: str) -> str:
    for item in items:
        if item == target:
            return item
    return ""
"#,
        &["return \"\".to_string()"],
    );
}

#[test]
fn test_vec_string_type() {
    transpile_and_check(
        r#"
def process_names(names: list[str]) -> list[str]:
    return names
"#,
        &["names: Vec<String>) -> Vec<String>"],
    );
}

#[test]
fn test_hashmap_string_key() {
    let rust_code = transpile(
        r#"
def get_value(d: dict[str, int], key: str) -> int:
    return d[key]
"#,
    );
    assert!(
        rust_code.contains("HashMap<String, i32>") && rust_code.contains("-> i32"),
        "\n{rust_code}"
    );
    assert!(
        rust_code.contains("d.get(&key)") || rust_code.contains("d[&key]"),
        "\n{rust_code}"
    );
}

#[test]
fn test_chained_string_methods() {
    transpile_and_check(
        r#"
def normalize(s: str) -> str:
    return s.strip().lower()
"#,
        &["s.trim().to_lowercase()"],
    );
}

#[test]
fn test_deeply_nested_string_field_access() {
    transpile_and_check(
        r#"
@dataclass
class Inner:
    value: str

@dataclass
class Outer:
    inner: Inner

def get_nested_string(o: Outer) -> str:
    return o.inner.value
"#,
        &["o.inner.value.clone()"],
    );
}

#[test]
fn test_string_equality_comparison() {
    transpile_and_check(
        r#"
def are_equal(a: str, b: str) -> bool:
    return a == b
"#,
        &["a == b"],
    );
}

#[test]
fn test_string_inequality_comparison() {
    transpile_and_check(
        r#"
def are_different(a: str, b: str) -> bool:
    return a != b
"#,
        &["a != b"],
    );
}

#[test]
fn test_string_comparison_with_literal() {
    transpile_and_check(
        r#"
def is_empty_str(s: str) -> bool:
    return s == ""
"#,
        &["s == \"\""],
    );
}

#[test]
fn test_string_in_ternary_expression() {
    transpile_and_check(
        r#"
def pick_string(flag: bool) -> str:
    return "yes" if flag else "no"
"#,
        &["\"yes\".to_string()", "\"no\".to_string()"],
    );
}

#[test]
fn test_string_local_variable_assignment() {
    transpile_and_check(
        r#"
def create_message() -> str:
    msg: str = "hello"
    return msg
"#,
        &["let msg: String = \"hello\".to_string()"],
    );
}

#[test]
fn test_string_reassignment() {
    let rust_code = transpile(
        r#"
def modify_string(s: str) -> str:
    s = s + "!"
    s = s + "?"
    return s
"#,
    );
    assert!(
        rust_code.contains("let mut s = s") || rust_code.contains("mut s: String"),
        "\n{rust_code}"
    );
}

#[test]
fn test_string_in_tuple_return() {
    transpile_and_check(
        r#"
def split_pair(s: str) -> tuple[str, str]:
    return (s, s)
"#,
        &["-> (String, String)"],
    );
}

#[test]
fn test_string_tuple_with_clone() {
    let rust_code = transpile(
        r#"
def duplicate(s: str) -> tuple[str, str]:
    return (s, s)
"#,
    );
    assert!(
        rust_code.contains("(s.clone(), s)") || rust_code.contains("(s, s.clone())"),
        "\n{rust_code}"
    );
}

#[test]
fn test_hashmap_string_value() {
    transpile_and_check(
        r#"
def create_map() -> dict[int, str]:
    return {1: "one", 2: "two"}
"#,
        &["HashMap<i32, String>", "\"one\".to_string()"],
    );
}

#[test]
fn test_hashmap_string_key_literal() {
    transpile_and_check(
        r#"
def create_dict() -> dict[str, int]:
    return {"a": 1, "b": 2}
"#,
        &["(\"a\".to_string(), 1)", "(\"b\".to_string(), 2)"],
    );
}

#[test]
fn test_set_of_strings() {
    transpile_and_check(
        r#"
def unique_strings(items: list[str]) -> set[str]:
    return set(items)
"#,
        &["HashSet<String>"],
    );
}

#[test]
fn test_fstring_multiple_args() {
    transpile_and_check(
        r#"
def format_full(first: str, last: str, age: int) -> str:
    return f"{first} {last} is {age} years old"
"#,
        &["format!(\"{} {} is {} years old\", first, last, age)"],
    );
}

#[test]
fn test_fstring_with_expression() {
    transpile_and_check(
        r#"
def format_computed(n: int) -> str:
    return f"double is {n * 2}"
"#,
        &["format!(\"double is {}\", n * 2)"],
    );
}

#[test]
fn test_string_len() {
    transpile_and_check(
        r#"
def string_length(s: str) -> int:
    return len(s)
"#,
        &["s.len() as i32"],
    );
}

#[test]
fn test_empty_string_check_via_len() {
    let rust_code = transpile(
        r#"
def is_empty(s: str) -> bool:
    return len(s) == 0
"#,
    );
    assert!(
        rust_code.contains("s.is_empty()")
            || rust_code.contains("s.len() == 0")
            || rust_code.contains("s.len() as i32 == 0"),
        "\n{rust_code}"
    );
}

#[test]
fn test_string_not_in_check() {
    transpile_and_check(
        r#"
def lacks_substring(s: str, sub: str) -> bool:
    return sub not in s
"#,
        &["!s.contains(&sub)"],
    );
}

#[test]
fn test_string_list_iteration() {
    transpile_and_check(
        r#"
def process_all(items: list[str]) -> list[str]:
    result: list[str] = []
    for item in items:
        result.append(item.upper())
    return result
"#,
        &["for item in items", "item.to_uppercase()"],
    );
}

#[test]
fn test_string_enumerate_iteration() {
    transpile_and_check(
        r#"
def with_index(items: list[str]) -> list[str]:
    result: list[str] = []
    for i, item in enumerate(items):
        result.append(f"{i}: {item}")
    return result
"#,
        &[".enumerate()", "format!(\"{}: {}\", i, item)"],
    );
}

#[test]
fn test_string_concatenation_multiple() {
    let rust_code = transpile(
        r#"
def concat_three(a: str, b: str, c: str) -> str:
    return a + b + c
"#,
    );
    assert!(
        rust_code.contains("a + &b + &c") || rust_code.contains("format!"),
        "\n{rust_code}"
    );
}

#[test]
fn test_string_field_to_field_assignment() {
    transpile_and_check(
        r#"
@dataclass
class Pair:
    left: str
    right: str

def copy_left_to_right(p: Pair) -> None:
    p.right = p.left
"#,
        &["p.right = p.left.clone()"],
    );
}

#[test]
fn test_string_param_passed_to_multiple_calls() {
    let rust_code = transpile(
        r#"
def use_string(s: str) -> None:
    pass

def call_multiple(s: str) -> None:
    use_string(s)
    use_string(s)
    use_string(s)
"#,
    );
    assert!(
        rust_code.contains("use_string(s.clone())")
            || rust_code.contains("use_string(s.to_string())")
            || rust_code.contains("s: String"),
        "\n{rust_code}"
    );
}

#[test]
fn test_string_in_list_comprehension() {
    transpile_and_check(
        r#"
def uppercase_all(items: list[str]) -> list[str]:
    return [s.upper() for s in items]
"#,
        &[".map(|s| s.to_uppercase())"],
    );
}

#[test]
fn test_string_filter_comprehension() {
    transpile_and_check(
        r#"
def non_empty(items: list[str]) -> list[str]:
    return [s for s in items if len(s) > 0]
"#,
        &[".filter("],
    );
}

#[test]
fn test_string_method_in_condition() {
    transpile_and_check(
        r#"
def check_prefix_conditional(s: str) -> str:
    if s.startswith("pre"):
        return "has prefix"
    return "no prefix"
"#,
        &["s.starts_with(\"pre\")"],
    );
}

#[test]
fn test_string_replace_with_variables() {
    transpile_and_check(
        r#"
def dynamic_replace(s: str, old: str, new: str) -> str:
    return s.replace(old, new)
"#,
        &["s.replace(&old, &new)"],
    );
}

#[test]
fn test_string_split_and_join_roundtrip() {
    transpile_and_check(
        r#"
def normalize_whitespace(s: str) -> str:
    parts = s.split(" ")
    return " ".join(parts)
"#,
        &["s.split(\" \")", "parts.join(\" \")"],
    );
}

#[test]
fn test_optional_string_unwrap_pattern() {
    transpile_and_check(
        r#"
from typing import Optional

def safe_upper(s: Optional[str]) -> str:
    if s is None:
        return ""
    return s.upper()
"#,
        &["Option<String>", "\"\".to_string()"],
    );
}

#[test]
fn test_string_builder_pattern() {
    transpile_and_check(
        r#"
def build_string(parts: list[str]) -> str:
    result: str = ""
    for part in parts:
        result = result + part
    return result
"#,
        &["let mut result: String"],
    );
}

#[test]
fn test_string_with_escaped_chars() {
    transpile_and_check(
        r#"
def with_newline() -> str:
    return "line1\nline2"
"#,
        &["\"line1\\nline2\".to_string()"],
    );
}

#[test]
fn test_string_with_tab() {
    transpile_and_check(
        r#"
def with_tab() -> str:
    return "col1\tcol2"
"#,
        &["\"col1\\tcol2\".to_string()"],
    );
}

#[test]
fn test_string_dataclass_field_type() {
    transpile_and_check(
        r#"
@dataclass
class Person:
    name: str
    email: str
"#,
        &["name: String", "email: String"],
    );
}

#[test]
fn test_string_dataclass_method_return() {
    transpile_and_check(
        r#"
@dataclass
class Greeter:
    name: str

    def get_name(self) -> str:
        return self.name
"#,
        &["fn get_name(&self) -> String", "self.name"],
    );
}

#[test]
fn test_string_dataclass_method_param() {
    transpile_and_check(
        r#"
@dataclass
class Buffer:
    content: str

    def append(self, suffix: str) -> None:
        self.content = self.content + suffix
"#,
        &["fn append(&mut self, suffix: String)"],
    );
}

#[test]
fn test_string_in_list_contains() {
    transpile_and_check(
        r#"
def has_word(words: list[str], word: str) -> bool:
    return word in words
"#,
        &["words.contains(&word)"],
    );
}

#[test]
fn test_dataclass_field_in_string_list_literal() {
    transpile_and_check(
        r#"
@dataclass
class State:
    x: str

def one(state: State) -> bool:
    return state.x in ["One", "Two"]
"#,
        &["[\"One\", \"Two\"].contains(&state.x.as_str())"],
    );
}

#[test]
fn test_string_list_append() {
    transpile_and_check(
        r#"
def add_item(items: list[str], item: str) -> None:
    items.append(item)
"#,
        &["items.push(item)"],
    );
}

#[test]
fn test_string_conditional_return() {
    transpile_and_check(
        r#"
def get_status(ok: bool) -> str:
    if ok:
        return "success"
    else:
        return "failure"
"#,
        &["\"success\".to_string()", "\"failure\".to_string()"],
    );
}

#[test]
fn test_string_early_return() {
    let rust_code = transpile(
        r#"
def validate(s: str) -> str:
    if len(s) == 0:
        return "empty"
    return s
"#,
    );
    assert!(rust_code.contains("return \"empty\".to_string()"), "\n{rust_code}");
    assert!(
        rust_code.contains("return s") || rust_code.contains("s\n"),
        "\n{rust_code}"
    );
}

#[test]
fn test_string_slice_equivalent() {
    transpile_and_check(
        r#"
def first_n_chars(s: str, n: int) -> str:
    return s[:n]
"#,
        &["fn first_n_chars(s: String, n: i32) -> String"],
    );
}

#[test]
fn test_string_used_after_method_call() {
    transpile_and_check(
        r#"
def transform_and_use(s: str) -> str:
    upper = s.upper()
    lower = s.lower()
    return upper + lower
"#,
        &["s.to_uppercase()", "s.to_lowercase()"],
    );
}

#[test]
fn test_string_literal_method_chain() {
    transpile_and_check(
        r#"
def get_processed() -> str:
    return "  HELLO  ".strip().lower()
"#,
        &["\"  HELLO  \".trim().to_lowercase()"],
    );
}

#[test]
fn test_nested_format_string() {
    transpile_and_check(
        r#"
def nested_format(s: str) -> str:
    return f"Result: {s.upper()}"
"#,
        &["format!(\"Result: {}\", s.to_uppercase())"],
    );
}

#[test]
fn test_string_default_param_simulation() {
    transpile_and_check(
        r#"
from typing import Optional

def greet_optional(name: Optional[str]) -> str:
    if name is None:
        return "Hello, World!"
    return f"Hello, {name}!"
"#,
        &["Option<String>", "\"Hello, World!\".to_string()"],
    );
}

#[test]
fn test_string_zip_iteration() {
    transpile_and_check(
        r#"
def pair_strings(a: list[str], b: list[str]) -> list[str]:
    result: list[str] = []
    for x, y in zip(a, b):
        result.append(x + y)
    return result
"#,
        &[".zip("],
    );
}

#[test]
fn test_string_any_predicate() {
    transpile_and_check(
        r#"
def any_starts_with(items: list[str], prefix: str) -> bool:
    return any(s.startswith(prefix) for s in items)
"#,
        &[".any("],
    );
}

#[test]
fn test_string_all_predicate() {
    transpile_and_check(
        r#"
def all_non_empty(items: list[str]) -> bool:
    return all(len(s) > 0 for s in items)
"#,
        &[".all("],
    );
}

#[test]
fn test_str_passing_between_functions() {
    // Comprehensive test: str passed as param, returned, assigned to variable, and passed again
    transpile_and_check(
        r#"
def process(s: str) -> str:
    return s.upper()

def transform(s: str) -> str:
    return s + "!"

def caller(input: str) -> str:
    processed: str = process(input)
    result: str = transform(processed)
    return result
"#,
        &[
            // All function signatures use String (not &str or str slice)
            "fn process(s: String) -> String",
            "fn transform(s: String) -> String",
            "fn caller(input: String) -> String",
            // Local variables storing str are typed as String
            "let processed: String",
            "let result: String",
        ],
    );
}

#[test]
fn test_str_flow_through_dataclass() {
    // Test: str flows from dataclass field -> function param -> return value
    transpile_and_check(
        r#"
@dataclass
class Container:
    value: str

def extract_upper(c: Container) -> str:
    return c.value.upper()

def process_container(c: Container, suffix: str) -> str:
    base: str = extract_upper(c)
    return base + suffix
"#,
        &[
            // Dataclass field is String
            "value: String",
            // Functions accept and return String
            "fn extract_upper(c: &Container) -> String",
            "fn process_container(c: &Container, suffix: String) -> String",
            // Local str variable is String
            "let base: String",
        ],
    );
}

#[test]
fn test_str_method_params_and_returns() {
    // Test: str used in method parameters and return types
    transpile_and_check(
        r#"
@dataclass
class Wrapper:
    prefix: str

    def wrap(self, value: str) -> str:
        return value.upper()

def use_wrapper(w: Wrapper, content: str) -> str:
    return w.wrap(content)
"#,
        &[
            // Dataclass field is String
            "prefix: String",
            // Method signature uses String
            "fn wrap(&self, value: String) -> String",
            // Function signature uses String
            "fn use_wrapper(w: &Wrapper, content: String) -> String",
        ],
    );
}

#[test]
fn test_str_in_multi_function_pipeline() {
    // Test: str passed through a pipeline of functions
    transpile_and_check(
        r#"
def step1(s: str) -> str:
    return s.strip()

def step2(s: str) -> str:
    return s.lower()

def step3(s: str) -> str:
    return s.replace(" ", "_")

def pipeline(input: str) -> str:
    a: str = step1(input)
    b: str = step2(a)
    c: str = step3(b)
    return c
"#,
        &[
            // All pipeline functions use String
            "fn step1(s: String) -> String",
            "fn step2(s: String) -> String",
            "fn step3(s: String) -> String",
            "fn pipeline(input: String) -> String",
            // Intermediate variables are String
            "let a: String",
            "let b: String",
            "let c: String",
        ],
    );
}

#[test]
fn test_str_field_passed_to_function_expecting_owned_string() {
    // Test: struct field of type str passed to function expecting str should NOT add & prefix
    // This was bug: assign_try(&state, index, &selection.team) instead of selection.team
    transpile_and_check(
        r#"
@dataclass
class Selection:
    player_index: int
    team: str

def process_team(team: str) -> None:
    print(team)

def use_selection(selection: Selection) -> None:
    process_team(selection.team)
"#,
        &[
            "fn process_team(team: String)",
            // Should pass selection.team directly (with clone), not &selection.team
            "process_team(selection.team.clone())",
        ],
    );
}

#[test]
fn test_fstring_optional_string_field() {
    transpile_and_check(
        r#"
@dataclass
class Person:
    name: Optional[str]

def format_name(p: Person) -> str:
    return f"Name: {p.name}"
"#,
        &[
            "name: Option<String>",
            "match &",
            "Some(v) => format!(\"{}\", v)",
            "None => \"None\".to_string()",
        ],
    );
}

#[test]
fn test_fstring_optional_int_field() {
    transpile_and_check(
        r#"
@dataclass
class Person:
    age: Optional[int]

def format_age(p: Person) -> str:
    return f"Age: {p.age}"
"#,
        &[
            "age: Option<i32>",
            "match &",
            "Some(v) => format!(\"{}\", v)",
            "None => \"None\".to_string()",
        ],
    );
}

#[test]
fn test_fstring_multiple_optional_fields() {
    transpile_and_check(
        r#"
@dataclass
class Person:
    name: Optional[str]
    age: Optional[int]

def format_person(p: Person) -> str:
    return f"{p.name} is {p.age} years old"
"#,
        &["name: Option<String>", "age: Option<i32>", "{} is {} years old"],
    );
}

#[test]
fn test_fstring_mixed_optional_and_required() {
    transpile_and_check(
        r#"
@dataclass
class Person:
    first_name: str
    nickname: Optional[str]

def format_mixed(p: Person) -> str:
    return f"{p.first_name} aka {p.nickname}"
"#,
        &[
            "first_name: String",
            "nickname: Option<String>",
            "format!(\"{} aka {}\",",
        ],
    );
}

#[test]
fn test_fstring_optional_variable() {
    transpile_and_check(
        r#"
def format_optional(name: Optional[str]) -> str:
    return f"Value: {name}"
"#,
        &[
            "Option<String>",
            "match &",
            "Some(v) => format!(\"{}\", v)",
            "None => \"None\".to_string()",
        ],
    );
}
