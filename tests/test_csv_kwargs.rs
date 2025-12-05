//! Tests for csv.DictWriter with keyword arguments

use crate::test_helpers;
use crate::test_helpers::transpile_and_check;

#[test]
#[ignore]
fn test_csv_dictwriter_basic() {
    let python = r#"
import csv

def write_data(filename):
    with open(filename, 'w') as f:
        writer = csv.DictWriter(f, fieldnames=['name', 'age'])
        writer.writeheader()
"#;
    transpile_and_check(python, &["csv::Writer"]);
}

#[test]
#[ignore]
fn test_csv_dictwriter_multiple_fields() {
    let python = r#"
import csv

writer = csv.DictWriter(output, fieldnames=['id', 'name', 'email', 'age'])
"#;
    transpile_and_check(python, &["csv"]);
}

#[test]
#[ignore]
fn test_csv_dictwriter_variable_fieldnames() {
    let python = r#"
import csv

writer = csv.DictWriter(f, fieldnames=fields)
"#;
    transpile_and_check(python, &["csv"]);
}

#[test]
#[ignore]
fn test_real_world_csv_filter() {
    let python = r#"
import csv
import sys

def filter_csv(input_file, column, value, output_file=None):
    with open(input_file, "r") as f:
        reader = csv.DictReader(f)
        fieldnames = reader.fieldnames
        filtered_rows = (row for row in reader if row[column] == value)

        output = open(output_file, "w") if output_file else sys.stdout

        try:
            writer = csv.DictWriter(output, fieldnames=fieldnames)
            writer.writeheader()

            for row in filtered_rows:
                writer.writerow(row)
        finally:
            if output_file:
                output.close()
"#;
    transpile_and_check(python, &["csv::Writer"]);
}

#[test]
#[ignore]
fn test_property_based_fieldnames() {
    let test_cases = [
        "csv.DictWriter(f, fieldnames=['a', 'b'])",
        "csv.DictWriter(f, fieldnames=fields)",
        "csv.DictWriter(f, fieldnames=reader.fieldnames)",
    ];

    for python_expr in test_cases {
        let python = format!("import csv\n{}", python_expr);
        transpile_and_check(&python, &["csv"]);
    }
}
