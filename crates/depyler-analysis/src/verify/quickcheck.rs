//! QuickCheck integration for property-based testing

use depyler_core::hir::Type;
use quickcheck::{Arbitrary, Gen};

pub struct TypedValue {
    pub ty: Type,
    pub value: serde_json::Value,
}

impl TypedValue {
    pub fn arbitrary_for_type(ty: &Type, g: &mut Gen) -> Self {
        let value = match ty {
            Type::Int => {
                let n: i32 = Arbitrary::arbitrary(g);
                serde_json::json!(n)
            }
            Type::Float => {
                let f: f64 = Arbitrary::arbitrary(g);
                let f = if f.is_finite() { f } else { 1.0 };
                serde_json::json!(f)
            }
            Type::String => {
                let s: String = Arbitrary::arbitrary(g);
                serde_json::json!(s)
            }
            Type::Bool => {
                let b: bool = Arbitrary::arbitrary(g);
                serde_json::json!(b)
            }
            Type::None => serde_json::Value::Null,
            Type::List(inner) => {
                let size = g.size();
                let len = (size % 10) + 1;
                let items: Vec<serde_json::Value> =
                    (0..len).map(|_| Self::arbitrary_for_type(inner, g).value).collect();
                serde_json::json!(items)
            }
            Type::Dict(key_ty, val_ty) => {
                let size = g.size();
                let len = (size % 5) + 1;
                let mut map = serde_json::Map::new();

                for i in 0..len {
                    let key = match key_ty.as_ref() {
                        Type::String => {
                            let s: String = Arbitrary::arbitrary(g);
                            s
                        }
                        Type::Int => {
                            let n: i32 = Arbitrary::arbitrary(g);
                            n.to_string()
                        }
                        _ => format!("key_{i}"),
                    };
                    let val = Self::arbitrary_for_type(val_ty, g).value;
                    map.insert(key, val);
                }
                serde_json::Value::Object(map)
            }
            Type::Optional(inner) => {
                let is_some: bool = Arbitrary::arbitrary(g);
                if is_some {
                    Self::arbitrary_for_type(inner, g).value
                } else {
                    serde_json::Value::Null
                }
            }
            Type::Tuple(types) => {
                let items: Vec<serde_json::Value> =
                    types.iter().map(|t| Self::arbitrary_for_type(t, g).value).collect();
                serde_json::json!(items)
            }
            _ => serde_json::Value::Null,
        };

        TypedValue { ty: ty.clone(), value }
    }

    pub fn shrink(&self) -> Vec<TypedValue> {
        let shrunk_values = shrink_value(&self.value, &self.ty);
        shrunk_values
            .into_iter()
            .map(|v| TypedValue {
                ty: self.ty.clone(),
                value: v,
            })
            .collect()
    }
}

fn shrink_value(value: &serde_json::Value, ty: &Type) -> Vec<serde_json::Value> {
    match (value, ty) {
        (serde_json::Value::Number(n), Type::Int) => {
            if let Some(i) = n.as_i64() {
                shrink_integer(i)
            } else {
                vec![]
            }
        }
        (serde_json::Value::Number(n), Type::Float) => {
            if let Some(f) = n.as_f64() {
                shrink_float(f)
            } else {
                vec![]
            }
        }
        (serde_json::Value::String(s), Type::String) => shrink_string(s),
        (serde_json::Value::Array(arr), Type::List(inner)) => shrink_array(arr, inner),
        _ => vec![],
    }
}

fn shrink_integer(i: i64) -> Vec<serde_json::Value> {
    if i == 0 {
        return vec![];
    }

    let mut shrunk = vec![serde_json::json!(0), serde_json::json!(i / 2)];

    if i > 0 {
        shrunk.push(serde_json::json!(i - 1));
    } else {
        shrunk.push(serde_json::json!(i + 1));
    }

    shrunk
}

fn shrink_float(f: f64) -> Vec<serde_json::Value> {
    if f == 0.0 {
        return vec![];
    }

    vec![
        serde_json::json!(0.0),
        serde_json::json!(f / 2.0),
        serde_json::json!(f.trunc()),
    ]
}

fn shrink_string(s: &str) -> Vec<serde_json::Value> {
    if s.is_empty() {
        return vec![];
    }

    let mut shrunk = vec![serde_json::json!("")];

    if s.len() > 1 {
        shrunk.push(serde_json::json!(&s[..s.len() / 2]));
        shrunk.push(serde_json::json!(&s[1..]));
    }

    shrunk
}

fn shrink_array(arr: &[serde_json::Value], inner: &Type) -> Vec<serde_json::Value> {
    if arr.is_empty() {
        return vec![];
    }

    let mut shrunk = vec![serde_json::json!([])];

    // Remove one element at a time
    for i in 0..arr.len() {
        let mut new_arr: Vec<_> = arr.iter().cloned().collect();
        new_arr.remove(i);
        shrunk.push(serde_json::json!(new_arr));
    }

    // Shrink each element
    for (i, elem) in arr.iter().enumerate() {
        for shrunk_elem in shrink_value(elem, inner) {
            let mut new_arr: Vec<_> = arr.iter().cloned().collect();
            new_arr[i] = shrunk_elem;
            shrunk.push(serde_json::json!(new_arr));
        }
    }

    shrunk
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shrink_integer() {
        let shrunk = shrink_integer(10);
        assert!(shrunk.contains(&serde_json::json!(0)));
        assert!(shrunk.contains(&serde_json::json!(5)));
    }

    #[test]
    fn test_shrink_string() {
        let shrunk = shrink_string("hello");
        assert!(shrunk.contains(&serde_json::json!("")));
    }
}
