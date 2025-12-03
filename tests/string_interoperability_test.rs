use depyler_core::DepylerPipeline;

#[test]
fn test_str_param_becomes_owned_string() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def greet(name: str) -> str:
    return name
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("fn greet(name: String) -> String"), "\n{rust_code}");
}

#[test]
fn test_string_literal_to_string_conversion() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def get_greeting() -> str:
    return "hello"
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("\"hello\".to_string()"), "\n{rust_code}");
}

#[test]
fn test_string_field_mutation_requires_to_string() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
@dataclass
class State:
    value: str

def set_value(state: State) -> None:
    state.value = "updated"
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(
        rust_code.contains("state.value = \"updated\".to_string()"),
        "\n{rust_code}"
    );
}

#[test]
fn test_string_clone_on_field_access() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
@dataclass
class Container:
    text: str

def get_text(c: Container) -> str:
    return c.text
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("c.text.clone()"), "\n{rust_code}");
}

#[test]
fn test_string_clone_when_reused() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def take_string(s: str) -> str:
    return s + "!"

def use_twice(s: str) -> str:
    first = take_string(s)
    second = take_string(s)
    return first + second
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(
        rust_code.contains("take_string(s.clone())")
            || rust_code.contains("take_string(s.to_string())")
            || rust_code.contains("s: &str"),
        "\n{rust_code}"
    );
}

#[test]
fn test_starts_with_takes_str_ref() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def check_prefix(s: str, prefix: str) -> bool:
    return s.startswith(prefix)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("s.starts_with(&prefix)"), "\n{rust_code}");
}

#[test]
fn test_ends_with_takes_str_ref() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def check_suffix(s: str, suffix: str) -> bool:
    return s.endswith(suffix)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("s.ends_with(&suffix)"), "\n{rust_code}");
}

#[test]
fn test_contains_takes_str_ref() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def has_substring(s: str, sub: str) -> bool:
    return sub in s
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("s.contains(&sub)"), "\n{rust_code}");
}

#[test]
fn test_join_takes_str_ref() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def join_parts(parts: list[str], sep: str) -> str:
    return sep.join(parts)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("parts.join(&sep)"), "\n{rust_code}");
}

#[test]
fn test_split_returns_vec_string() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def split_csv(s: str) -> list[str]:
    return s.split(",")
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("-> Vec<String>"), "\n{rust_code}");
    assert!(rust_code.contains(".map(|s| s.to_string()).collect()"), "\n{rust_code}");
}

#[test]
fn test_string_method_chain_ownership() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def clean(s: str) -> str:
    return s.strip()
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("s.trim()"), "\n{rust_code}");
}

#[test]
fn test_case_methods_return_string() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def to_lower(s: str) -> str:
    return s.lower()

def to_upper(s: str) -> str:
    return s.upper()
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("s.to_lowercase()"), "\n{rust_code}");
    assert!(rust_code.contains("s.to_uppercase()"), "\n{rust_code}");
}

#[test]
fn test_replace_with_str_literals() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def replace_spaces(s: str) -> str:
    return s.replace(" ", "_")
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("s.replace(\" \", \"_\")"), "\n{rust_code}");
}

#[test]
fn test_fstring_format_macro() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def format_name(name: str) -> str:
    return f"Hello, {name}!"
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("format!(\"Hello, {}!\", name)"), "\n{rust_code}");
}

#[test]
fn test_string_concatenation() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
@dataclass
class State:
    a: str
    b: str

def concat_fields(state: State) -> None:
    state.b = state.a + "suffix"
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(
        rust_code.contains("format!") && rust_code.contains("state.a"),
        "\n{rust_code}"
    );
}

#[test]
fn test_optional_string() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
from typing import Optional

def unwrap_or_default(val: Optional[str], default: str) -> str:
    if val is None:
        return default
    return val
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(
        rust_code.contains("Option<String>") && rust_code.contains("-> String"),
        "\n{rust_code}"
    );
}

#[test]
fn test_string_repeat() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def repeat_string(s: str, n: int) -> str:
    return s * n
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("s.repeat(n as usize)"), "\n{rust_code}");
}

#[test]
fn test_int_to_string_conversion() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def int_to_string(n: int) -> str:
    return str(n)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("n.to_string()"), "\n{rust_code}");
}

#[test]
fn test_string_to_int_parse() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def parse_int(s: str) -> int:
    return int(s)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("s.parse::<i32>().unwrap()"), "\n{rust_code}");
}

#[test]
fn test_method_on_string_literal() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def get_upper() -> str:
    return "hello".upper()
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("\"hello\".to_uppercase()"), "\n{rust_code}");
}

#[test]
fn test_empty_string_literal() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def find_first(items: list[str], target: str) -> str:
    for item in items:
        if item == target:
            return item
    return ""
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("return \"\".to_string()"), "\n{rust_code}");
}

#[test]
fn test_vec_string_type() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def process_names(names: list[str]) -> list[str]:
    return names
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(
        rust_code.contains("names: Vec<String>) -> Vec<String>"),
        "\n{rust_code}"
    );
}

#[test]
fn test_hashmap_string_key() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def get_value(d: dict[str, int], key: str) -> int:
    return d[key]
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
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
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def normalize(s: str) -> str:
    return s.strip().lower()
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("s.trim().to_lowercase()"), "\n{rust_code}");
}

#[test]
fn test_deeply_nested_string_field_access() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
@dataclass
class Inner:
    value: str

@dataclass
class Outer:
    inner: Inner

def get_nested_string(o: Outer) -> str:
    return o.inner.value
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("o.inner.value.clone()"), "\n{rust_code}");
}

#[test]
fn test_string_equality_comparison() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def are_equal(a: str, b: str) -> bool:
    return a == b
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("a == b"), "\n{rust_code}");
}

#[test]
fn test_string_inequality_comparison() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def are_different(a: str, b: str) -> bool:
    return a != b
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("a != b"), "\n{rust_code}");
}

#[test]
fn test_string_comparison_with_literal() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def is_empty_str(s: str) -> bool:
    return s == ""
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("s == \"\""), "\n{rust_code}");
}

#[test]
fn test_string_in_ternary_expression() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def pick_string(flag: bool) -> str:
    return "yes" if flag else "no"
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("\"yes\".to_string()"), "\n{rust_code}");
    assert!(rust_code.contains("\"no\".to_string()"), "\n{rust_code}");
}

#[test]
fn test_string_local_variable_assignment() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def create_message() -> str:
    msg: str = "hello"
    return msg
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(
        rust_code.contains("let msg: String = \"hello\".to_string()"),
        "\n{rust_code}"
    );
}

#[test]
fn test_string_reassignment() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def modify_string(s: str) -> str:
    s = s + "!"
    s = s + "?"
    return s
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(
        rust_code.contains("let mut s = s") || rust_code.contains("mut s: String"),
        "\n{rust_code}"
    );
}

#[test]
fn test_string_in_tuple_return() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def split_pair(s: str) -> tuple[str, str]:
    return (s, s)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("-> (String, String)"), "\n{rust_code}");
}

#[test]
fn test_string_tuple_with_clone() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def duplicate(s: str) -> tuple[str, str]:
    return (s, s)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(
        rust_code.contains("(s.clone(), s)") || rust_code.contains("(s, s.clone())"),
        "\n{rust_code}"
    );
}

#[test]
fn test_hashmap_string_value() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def create_map() -> dict[int, str]:
    return {1: "one", 2: "two"}
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("HashMap<i32, String>"), "\n{rust_code}");
    assert!(rust_code.contains("\"one\".to_string()"), "\n{rust_code}");
}

#[test]
fn test_hashmap_string_key_literal() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def create_dict() -> dict[str, int]:
    return {"a": 1, "b": 2}
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("(\"a\".to_string(), 1)"), "\n{rust_code}");
    assert!(rust_code.contains("(\"b\".to_string(), 2)"), "\n{rust_code}");
}

#[test]
fn test_set_of_strings() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def unique_strings(items: list[str]) -> set[str]:
    return set(items)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("HashSet<String>"), "\n{rust_code}");
}

#[test]
fn test_fstring_multiple_args() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def format_full(first: str, last: str, age: int) -> str:
    return f"{first} {last} is {age} years old"
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(
        rust_code.contains("format!(\"{} {} is {} years old\", first, last, age)"),
        "\n{rust_code}"
    );
}

#[test]
fn test_fstring_with_expression() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def format_computed(n: int) -> str:
    return f"double is {n * 2}"
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("format!(\"double is {}\", n * 2)"), "\n{rust_code}");
}

#[test]
fn test_string_len() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def string_length(s: str) -> int:
    return len(s)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("s.len() as i32"), "\n{rust_code}");
}

#[test]
fn test_empty_string_check_via_len() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def is_empty(s: str) -> bool:
    return len(s) == 0
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(
        rust_code.contains("s.is_empty()")
            || rust_code.contains("s.len() == 0")
            || rust_code.contains("s.len() as i32 == 0"),
        "\n{rust_code}"
    );
}

#[test]
fn test_string_not_in_check() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def lacks_substring(s: str, sub: str) -> bool:
    return sub not in s
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("!s.contains(&sub)"), "\n{rust_code}");
}

#[test]
fn test_string_list_iteration() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def process_all(items: list[str]) -> list[str]:
    result: list[str] = []
    for item in items:
        result.append(item.upper())
    return result
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("for item in items"), "\n{rust_code}");
    assert!(rust_code.contains("item.to_uppercase()"), "\n{rust_code}");
}

#[test]
fn test_string_enumerate_iteration() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def with_index(items: list[str]) -> list[str]:
    result: list[str] = []
    for i, item in enumerate(items):
        result.append(f"{i}: {item}")
    return result
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains(".enumerate()"), "\n{rust_code}");
    assert!(rust_code.contains("format!(\"{}: {}\", i, item)"), "\n{rust_code}");
}

#[test]
fn test_string_concatenation_multiple() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def concat_three(a: str, b: str, c: str) -> str:
    return a + b + c
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(
        rust_code.contains("a + &b + &c") || rust_code.contains("format!"),
        "\n{rust_code}"
    );
}

#[test]
fn test_string_field_to_field_assignment() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
@dataclass
class Pair:
    left: str
    right: str

def copy_left_to_right(p: Pair) -> None:
    p.right = p.left
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("p.right = p.left.clone()"), "\n{rust_code}");
}

#[test]
fn test_string_param_passed_to_multiple_calls() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def use_string(s: str) -> None:
    pass

def call_multiple(s: str) -> None:
    use_string(s)
    use_string(s)
    use_string(s)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(
        rust_code.contains("use_string(s.clone())")
            || rust_code.contains("use_string(s.to_string())")
            || rust_code.contains("s: &str"),
        "\n{rust_code}"
    );
}

#[test]
fn test_string_in_list_comprehension() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def uppercase_all(items: list[str]) -> list[str]:
    return [s.upper() for s in items]
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains(".map(|s| s.to_uppercase())"), "\n{rust_code}");
}

#[test]
fn test_string_filter_comprehension() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def non_empty(items: list[str]) -> list[str]:
    return [s for s in items if len(s) > 0]
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains(".filter("), "\n{rust_code}");
}

#[test]
fn test_string_method_in_condition() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def check_prefix_conditional(s: str) -> str:
    if s.startswith("pre"):
        return "has prefix"
    return "no prefix"
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("s.starts_with(\"pre\")"), "\n{rust_code}");
}

#[test]
fn test_string_replace_with_variables() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def dynamic_replace(s: str, old: str, new: str) -> str:
    return s.replace(old, new)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("s.replace(&old, &new)"), "\n{rust_code}");
}

#[test]
fn test_string_split_and_join_roundtrip() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def normalize_whitespace(s: str) -> str:
    parts = s.split(" ")
    return " ".join(parts)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("s.split(\" \")"), "\n{rust_code}");
    assert!(rust_code.contains("parts.join(\" \")"), "\n{rust_code}");
}

#[test]
fn test_optional_string_unwrap_pattern() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
from typing import Optional

def safe_upper(s: Optional[str]) -> str:
    if s is None:
        return ""
    return s.upper()
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("Option<String>"), "\n{rust_code}");
    assert!(rust_code.contains("\"\".to_string()"), "\n{rust_code}");
}

#[test]
fn test_string_builder_pattern() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def build_string(parts: list[str]) -> str:
    result: str = ""
    for part in parts:
        result = result + part
    return result
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("let mut result: String"), "\n{rust_code}");
}

#[test]
fn test_string_with_escaped_chars() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def with_newline() -> str:
    return "line1\nline2"
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("\"line1\\nline2\".to_string()"), "\n{rust_code}");
}

#[test]
fn test_string_with_tab() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def with_tab() -> str:
    return "col1\tcol2"
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("\"col1\\tcol2\".to_string()"), "\n{rust_code}");
}

#[test]
fn test_string_dataclass_field_type() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
@dataclass
class Person:
    name: str
    email: str
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("name: String"), "\n{rust_code}");
    assert!(rust_code.contains("email: String"), "\n{rust_code}");
}

#[test]
fn test_string_dataclass_method_return() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
@dataclass
class Greeter:
    name: str

    def get_name(self) -> str:
        return self.name
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("fn get_name(&self) -> String"), "\n{rust_code}");
    assert!(rust_code.contains("self.name"), "\n{rust_code}");
}

#[test]
fn test_string_dataclass_method_param() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
@dataclass
class Buffer:
    content: str

    def append(self, suffix: str) -> None:
        self.content = self.content + suffix
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(
        rust_code.contains("fn append(&mut self, suffix: String)"),
        "\n{rust_code}"
    );
}

#[test]
fn test_string_in_list_contains() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def has_word(words: list[str], word: str) -> bool:
    return word in words
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("words.contains(&word)"), "\n{rust_code}");
}

#[test]
fn test_string_list_append() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def add_item(items: list[str], item: str) -> None:
    items.append(item)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("items.push(item)"), "\n{rust_code}");
}

#[test]
fn test_string_conditional_return() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def get_status(ok: bool) -> str:
    if ok:
        return "success"
    else:
        return "failure"
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("\"success\".to_string()"), "\n{rust_code}");
    assert!(rust_code.contains("\"failure\".to_string()"), "\n{rust_code}");
}

#[test]
fn test_string_early_return() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def validate(s: str) -> str:
    if len(s) == 0:
        return "empty"
    return s
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("return \"empty\".to_string()"), "\n{rust_code}");
    assert!(
        rust_code.contains("return s") || rust_code.contains("s\n"),
        "\n{rust_code}"
    );
}

#[test]
fn test_string_slice_equivalent() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def first_n_chars(s: str, n: int) -> str:
    return s[:n]
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(
        rust_code.contains("fn first_n_chars(s: String, n: i32) -> String"),
        "\n{rust_code}"
    );
}

#[test]
fn test_string_used_after_method_call() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def transform_and_use(s: str) -> str:
    upper = s.upper()
    lower = s.lower()
    return upper + lower
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("s.to_uppercase()"), "\n{rust_code}");
    assert!(rust_code.contains("s.to_lowercase()"), "\n{rust_code}");
}

#[test]
fn test_string_literal_method_chain() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def get_processed() -> str:
    return "  HELLO  ".strip().lower()
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(
        rust_code.contains("\"  HELLO  \".trim().to_lowercase()"),
        "\n{rust_code}"
    );
}

#[test]
fn test_nested_format_string() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def nested_format(s: str) -> str:
    return f"Result: {s.upper()}"
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(
        rust_code.contains("format!(\"Result: {}\", s.to_uppercase())"),
        "\n{rust_code}"
    );
}

#[test]
fn test_string_default_param_simulation() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
from typing import Optional

def greet_optional(name: Optional[str]) -> str:
    if name is None:
        return "Hello, World!"
    return f"Hello, {name}!"
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains("Option<String>"), "\n{rust_code}");
    assert!(rust_code.contains("\"Hello, World!\".to_string()"), "\n{rust_code}");
}

#[test]
fn test_string_zip_iteration() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def pair_strings(a: list[str], b: list[str]) -> list[str]:
    result: list[str] = []
    for x, y in zip(a, b):
        result.append(x + y)
    return result
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains(".zip("), "\n{rust_code}");
}

#[test]
fn test_string_any_predicate() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def any_starts_with(items: list[str], prefix: str) -> bool:
    return any(s.startswith(prefix) for s in items)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains(".any("), "\n{rust_code}");
}

#[test]
fn test_string_all_predicate() {
    let pipeline = DepylerPipeline::new();
    let python_code = r#"
def all_non_empty(items: list[str]) -> bool:
    return all(len(s) > 0 for s in items)
"#;

    let rust_code = pipeline.transpile(python_code).unwrap();
    assert!(rust_code.contains(".all("), "\n{rust_code}");
}
