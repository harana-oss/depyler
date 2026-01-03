lazy_static! {
    pub static ref name: String = "Slicing Tests";
    pub static ref category: String = "slicing";
    pub static ref name: String = "slice_start_only";
    pub static ref description: String = "Slice with start index only(items[n:])";
    pub static ref python: String = "\ndef skip_first_n(items: list[int], n: int) -> list[int]:\n    return items[n:]\n";
    pub static ref rust: String = "\npub fn skip_first_n(items: &Vec<i32>, n: i32) -> Vec<i32>{\n    return {\n        let base = &items;\n        let start =(n).max(0) as usize;\n        if start<base.len() {\n            base[start..].to_vec()\n       
}
else {\n            Vec::new()\n        }\n    }\n";
    pub static ref name: String = "slice_end_only";
    pub static ref description: String = "Slice with end index only(items[:n])";
    pub static ref python: String = "\ndef first_n(items: list[int], n: int) -> list[int]:\n    return items[:n]\n";
    pub static ref rust: String = "\npub fn first_n(items: &Vec<i32>, n: i32) -> Vec<i32>{\n    return {\n        let base = &items;\n        let stop =(n).max(0) as usize;\n        base[..stop.min (base.len())].to_vec()\n    }\n";
    pub static ref name: String = "slice_start_end";
    pub static ref description: String = "Slice with both start and end(items[a:b])";
    pub static ref python: String = "\ndef middle(items: list[int], start: int, end: int) -> list[int]:\n    return items[start:end]\n";
    pub static ref rust: String = "\npub fn middle(items: &Vec<i32>, start: i32, end: i32) -> Vec<i32>{\n    return {\n        let base = &items;\n        let start =(start).max(0) as usize;\n        let stop =(end).max(0) as usize;\n        if start<base.len() {\n            base[start..stop.min (base.len())].to_vec()\n       
}
else {\n            Vec::new()\n        }\n    }\n";
    pub static ref name: String = "slice_full_copy";
    pub static ref description: String = "Full slice copy(items[:])";
    pub static ref python: String = "\ndef copy_list(items: list[int]) -> list[int]:\n    return items[:]\n";
    pub static ref rust: String = "\npub fn copy_list(items: &Vec<i32>) -> Vec<i32>{\n    return items.clone();\n}\n";
    pub static ref name: String = "index_negative_one";
    pub static ref description: String = "Access last element with -1";
    pub static ref python: String = "\ndef get_last(items: list[int]) -> int:\n    return items[-1]\n";
    pub static ref rust: String = "\n#[derive(Debug, Clone)]\npub struct IndexError {\n    message: String,\n}\nimpl std::fmt::Display for IndexError {\n    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n        write!(f, \"index out of range: {}\", self.message)\n    }\n}\nimpl std::error::Error for IndexError {}\nimpl IndexError {\n    pub fn new(message: impl Into<String>) -> Self {\n        Self {\n            message: message.into(),\n        }\n    }\n}\n#[doc = \" Depyler: proven to terminate\"]\npub fn get_last(items: &Vec<i32>) -> i32 {\n    return items.last().cloned().unwrap();\n}\n";
    pub static ref name: String = "slice_negative_start";
    pub static ref description: String = "Slice with negative start(items[-n:])";
    pub static ref python: String = "\ndef last_n(items: list[int], n: int) -> list[int]:\n    return items[-n:]\n";
    pub static ref rust: String = "\npub fn last_n(items: &Vec<i32>, n: i32) -> Vec<i32>{\n    return {\n        let base = &items;\n        let start =(-n).max(0) as usize;\n        if start<base.len() {\n            base[start..].to_vec()\n       
}
else {\n            Vec::new()\n        }\n    }\n";
    pub static ref name: String = "slice_negative_end";
    pub static ref description: String = "Slice with negative end(items[:-n])";
    pub static ref python: String = "\ndef all_but_last_n(items: list[int], n: int) -> list[int]:\n    return items[:-n]\n";
    pub static ref rust: String = "\npub fn all_but_last_n(items: &Vec<i32>, n: i32) -> Vec<i32>{\n    return {\n        let base = &items;\n        let stop =(-n).max(0) as usize;\n        base[..stop.min (base.len())].to_vec()\n    }\n";
    pub static ref name: String = "slice_step";
    pub static ref description: String = "Slice with step(items[::n])";
    pub static ref python: String = "\ndef every_nth(items: list[int], n: int) -> list[int]:\n    return items[::n]\n";
    pub static ref rust: String = "\npub fn every_nth(items: &Vec<i32>, n: i32) -> Vec<i32>{\n    return {\n        let base = &items;\n        let step = n;\n        if step == 1 {\n            base.clone()\n       
}
else if step>0 {\n            base.iter().step_by(step as usize).cloned().collect::<Vec<_>>()\n       
}
else if step == -1 {\n            base.iter().rev().cloned().collect::<Vec<_>>()\n       
}
else {\n            let abs_step =(-step) as usize;\n            base.iter().rev().step_by(abs_step).cloned().collect::<Vec<_>>()\n        }\n    }\n";
    pub static ref name: String = "slice_step_with_range";
    pub static ref description: String = "Slice with start, end, and step";
    pub static ref python: String = "\ndef range_step(items: list[int], start: int, end: int, step: int) -> list[int]:\n    return items[start:end:step]\n";
    pub static ref rust: String = "\npub fn range_step(items: &Vec<i32>, start: i32, end: i32, step: i32) -> Vec<i32>{\n    return {\n        let base = &items;\n        let start =(start).max(0) as usize;\n        let stop =(end).max(0) as usize;\n        let step = step;\n        if step == 1 {\n            if start<base.len() {\n                base[start..stop.min (base.len())].to_vec()\n           
}
else {\n                Vec::new()\n            }\n       
}
else if step>0 {\n            base[start..stop.min (base.len())]\n                .iter()\n                .step_by(step as usize)\n                .cloned()\n                .collect::<Vec<_>>()\n       
}
else {\n            let abs_step =(-step) as usize;\n            if start<base.len() {\n                base[start..stop.min (base.len())]\n                    .iter()\n                    .rev()\n                    .step_by(abs_step)\n                    .cloned()\n                    .collect::<Vec<_>>()\n           
}
else {\n                Vec::new()\n            }\n        }\n    }\n";
    pub static ref name: String = "slice_reverse";
    pub static ref description: String = "Reverse with [::-1]";
    pub static ref python: String = "\ndef reverse_list(items: list[int]) -> list[int]:\n    return items[::-1]\n";
    pub static ref rust: String = "\npub fn reverse_list(items: &Vec<i32>) -> Vec<i32>{\n    return {\n        let base = &items;\n        let step = -1;\n        if step == 1 {\n            base.clone()\n       
}
else if step>0 {\n            base.iter().step_by(step as usize).cloned().collect::<Vec<_>>()\n       
}
else if step == -1 {\n            base.iter().rev().cloned().collect::<Vec<_>>()\n       
}
else {\n            let abs_step =(-step) as usize;\n            base.iter().rev().step_by(abs_step).cloned().collect::<Vec<_>>()\n        }\n    };\n}\n";
    pub static ref name: String = "string_slice_start";
    pub static ref description: String = "String slice from start";
    pub static ref python: String = "\ndef skip_chars(s: str, n: int) -> str:\n    return s[n:]\n";
    pub static ref rust: String = "\npub fn skip_chars(s: String, n: i32) -> String {\n    return {\n        let base = &s;\n        let start_idx: i32 = n;\n        let len = base.chars().count() as i32;\n        let actual_start = if start_idx<0 {\n           (len + start_idx).max(0) as usize\n       
}
else {\n            start_idx.min (len) as usize\n        };\n        base.chars().skip(actual_start).collect::<String>()\n    }\n";
    pub static ref name: String = "string_slice_end";
    pub static ref description: String = "String slice to end";
    pub static ref python: String = "\ndef first_chars(s: str, n: int) -> str:\n    return s[:n]\n";
    pub static ref rust: String = "\npub fn first_chars(s: String, n: i32) -> String {\n    return {\n        let base = &s;\n        let stop_idx: i32 = n;\n        let len = base.chars().count() as i32;\n        let actual_stop = if stop_idx<0 {\n           (len + stop_idx).max(0) as usize\n       
}
else {\n            stop_idx.min (len) as usize\n        };\n        base.chars().take(actual_stop).collect::<String>()\n    }\n";
    pub static ref name: String = "slice_assign_basic";
    pub static ref description: String = "Basic slice assignment";
    pub static ref python: String = "\ndef replace_middle(items: list[int], start: int, end: int, new: list[int]):\n    items[start:end] = new\n";
    pub static ref rust: String = "\npub fn replace_middle(mut items: Vec<i32>, start: i32, end: i32, new: &Vec<i32>) {\n    items.clear();\n    items.extend(new);\n}\n";
    pub static ref name: String = "slice_with_step";
    pub static ref description: String = "Slice with step using step_by";
    pub static ref python: String = "\ndef slice_test():\n    arr = [1, 2, 3, 4, 5, 6]\n    return arr[::2]\n";
    pub static ref rust: String = "\npub fn slice_test() {\n    let arr = vec![1, 2, 3, 4, 5, 6];\n    return {\n        let base = &arr;\n        let step = 2;\n        if step == 1 {\n            base.clone()\n       
}
else if step>0 {\n            base.iter().step_by(step as usize).cloned().collect::<Vec<_>>()\n       
}
else if step == -1 {\n            base.iter().rev().cloned().collect::<Vec<_>>()\n       
}
else {\n            let abs_step =(-step) as usize;\n            base.iter().rev().step_by(abs_step).cloned().collect::<Vec<_>>()\n        }\n    }\n";
    pub static ref name: String = "slice_negative_step";
    pub static ref description: String = "Reverse slice with negative step";
    pub static ref python: String = "\ndef reverse_slice():\n    arr = [1, 2, 3, 4, 5]\n    return arr[::-1]\n";
    pub static ref rust: String = "\npub fn reverse_slice() {\n    let arr = vec![1, 2, 3, 4, 5];\n    return {\n        let base = &arr;\n        let step = -1;\n        if step == 1 {\n            base.clone()\n       
}
else if step>0 {\n            base.iter().step_by(step as usize).cloned().collect::<Vec<_>>()\n       
}
else if step == -1 {\n            base.iter().rev().cloned().collect::<Vec<_>>()\n       
}
else {\n            let abs_step =(-step) as usize;\n            base.iter().rev().step_by(abs_step).cloned().collect::<Vec<_>>()\n        }\n    }\n";
   
}
use lazy_static::lazy_static;
    fn main () {
    vec! [metadata];
    vec! [vec! [test]];
    vec! [vec! [test]];
    vec! [vec! [test]];
    vec! [vec! [test]];
    vec! [vec! [test]];
    vec! [vec! [test]];
    vec! [vec! [test]];
    vec! [vec! [test]];
    vec! [vec! [test]];
    vec! [vec! [test]];
    vec! [vec! [test]];
    vec! [vec! [test]];
    vec! [vec! [test]];
    vec! [vec! [test]];
    vec! [vec! [test]];
    }