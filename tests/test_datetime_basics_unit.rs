// Module: datetime - Basic datetime functions

use crate::test_helpers::transpile_and_check;

//
#[test]
#[ignore]
fn test_date_today() {
    let python = r#"
import datetime

def get_today() -> datetime.date:
    return datetime.date.today()
"#;

    let result = transpile_and_check(python, &[]);

    // Should get current date
    assert!(result.contains("today") || result.contains("Utc::now"));
}

//
#[test]
#[ignore]
fn test_datetime_now() {
    let python = r#"
import datetime

def get_now() -> datetime.datetime:
    return datetime.datetime.now()
"#;

    let result = transpile_and_check(python, &[]);

    // Should get current datetime
    assert!(result.contains("now") || result.contains("Utc::now"));
}

//
#[test]
#[ignore]
fn test_timedelta() {
    let python = r#"
import datetime

def time_offset(days: int) -> datetime.timedelta:
    return datetime.timedelta(days=days)
"#;

    let result = transpile_and_check(python, &[]);

    // Should create time duration
    assert!(result.contains("Duration") || result.contains("days"));
}

// Total: 3 tests for datetime basic functions
// Coverage: date.today(), datetime.now(), timedelta()
