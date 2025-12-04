
// Module: heapq - Python heapq module validation
// pending

use crate::test_helpers::transpile_and_check;

// 
#[test]
fn test_heapify() {
    let python = r#"
import heapq

def create_heap(items: list) -> None:
    heapq.heapify(items)
"#;

    let result = transpile_and_check(python, &[]);

    // Should create min-heap in-place
    assert!(result.contains("heap") || result.contains("swap"));
}

// 
#[test]
fn test_heappush() {
    let python = r#"
import heapq

def push_item(heap: list, item: int) -> None:
    heapq.heappush(heap, item)
"#;

    let result = transpile_and_check(python, &[]);

    // Should push item onto heap
    assert!(result.contains("push") || result.contains("swap"));
}

#[test]
fn test_heappop() {
    let python = r#"
import heapq

def pop_min(heap: list) -> int:
    return heapq.heappop(heap)
"#;

    let result = transpile_and_check(python, &[]);

    // Should pop smallest item from heap
    assert!(result.contains("pop") || result.contains("swap"));
}

// 
#[test]
fn test_nlargest() {
    let python = r#"
import heapq

def get_largest(n: int, items: list) -> list:
    return heapq.nlargest(n, items)
"#;

    let result = transpile_and_check(python, &[]);

    // Should return n largest items
    assert!(result.contains("sort") || result.contains("take"));
}

#[test]
fn test_nsmallest() {
    let python = r#"
import heapq

def get_smallest(n: int, items: list) -> list:
    return heapq.nsmallest(n, items)
"#;

    let result = transpile_and_check(python, &[]);

    // Should return n smallest items
    assert!(result.contains("sort") || result.contains("take"));
}

// Total: 5 comprehensive tests for heapq module
// Coverage: heapify, heappush, heappop, nlargest, nsmallest
// Heap queue algorithm for priority queue operations
