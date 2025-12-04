//! Tests for CSV module code generation

use crate::test_helpers;
use crate::test_helpers::transpile_and_check;

#[test]
fn test_csv_dictreader_creation() {
    let python = r#"
import csv

def read_csv(filepath):
    with open(filepath) as f:
        reader = csv.DictReader(f)
        return list(reader)
"#;
    let rust_code = transpile_and_check(python, &["csv::ReaderBuilder", "deserialize"]);
    assert!(!rust_code.contains("reader.iter()"), "Should not use .iter() on csv::Reader");
}

#[test]
fn test_csv_fieldnames_access() {
    let python = r#"
import csv

def get_headers(filepath):
    with open(filepath) as f:
        reader = csv.DictReader(f)
        return reader.fieldnames
"#;
    let rust_code = transpile_and_check(python, &[".headers()"]);
    assert!(!rust_code.contains("reader.fieldnames"), "Should use .headers() not .fieldnames");
}

#[test]
fn test_csv_row_iteration() {
    let python = r#"
import csv

def print_rows(filepath):
    with open(filepath) as f:
        reader = csv.DictReader(f)
        for row in reader:
            print(row)
"#;
    let rust_code = transpile_and_check(python, &["deserialize", "HashMap<String, String>"]);
    assert!(!rust_code.contains("reader.iter()"), "Should not use .iter() on csv::Reader");
}

#[test]
fn test_csv_row_item_access() {
    let python = r#"
import csv

def get_column(filepath, column_name):
    with open(filepath) as f:
        reader = csv.DictReader(f)
        for row in reader:
            print(row[column_name])
"#;
    let rust_code = transpile_and_check(python, &[".get("]);
    assert!(!rust_code.contains("as usize"), "Should not use numeric indexing on HashMap");
}

#[test]
fn test_csv_filtering() {
    let python = r#"
import csv

def filter_csv(filepath, column, value):
    with open(filepath) as f:
        reader = csv.DictReader(f)
        results = []
        for row in reader:
            if row[column] == value:
                results.append(row)
        return results
"#;
    transpile_and_check(python, &["deserialize", "HashMap", ".get("]);
}

#[test]
fn test_csv_reader_generator_expression() {
    let python = r#"
import csv

def filter_csv_generator(filepath, column, value):
    with open(filepath) as f:
        reader = csv.DictReader(f)
        filtered = [row for row in reader if row[column] == value]
        return filtered
"#;
    let rust_code = transpile_and_check(python, &["deserialize", "HashMap"]);
    assert!(!rust_code.contains("reader.iter()"), "Should not use .iter() on csv::Reader");
}

#[test]
fn test_csv_reader_method_chain() {
    let python = r#"
import csv

def filter_and_map_csv(filepath):
    with open(filepath) as f:
        reader = csv.DictReader(f)
        names = [row['name'] for row in reader if row['active'] == 'true']
        return names
"#;
    let rust_code = transpile_and_check(python, &["deserialize"]);
    assert!(!rust_code.contains("reader.iter()"), "Should not use .iter() on csv::Reader");
}

#[test]
fn test_file_line_iteration() {
    let python = r#"
def read_lines(filepath):
    with open(filepath) as f:
        for line in f:
            print(line.strip())
"#;
    transpile_and_check(python, &["BufReader", ".lines()"]);
}
