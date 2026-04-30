//! Value types for shell variables.

use std::fmt;

/// A value in the shell.
///
/// This represents the different types of values that shell variables can hold.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// A string value
    String(String),
    /// An integer value
    Integer(i64),
    /// A floating-point value
    Float(f64),
    /// An array of values
    Array(Vec<Value>),
    /// An associative array (key-value pairs)
    AssocArray(std::collections::HashMap<String, Value>),
    /// A boolean value
    Boolean(bool),
    /// No value (unset)
    None,
}

impl Value {
    /// Create a string value.
    pub fn string(s: impl Into<String>) -> Self {
        Value::String(s.into())
    }

    /// Create an integer value.
    pub fn integer(n: i64) -> Self {
        Value::Integer(n)
    }

    /// Create a float value.
    pub fn float(f: f64) -> Self {
        Value::Float(f)
    }

    /// Create a boolean value.
    pub fn boolean(b: bool) -> Self {
        Value::Boolean(b)
    }

    /// Create an empty array.
    pub fn array() -> Self {
        Value::Array(Vec::new())
    }

    /// Create an array from a vector of values.
    pub fn array_from(values: Vec<Value>) -> Self {
        Value::Array(values)
    }

    /// Create an empty associative array.
    pub fn assoc_array() -> Self {
        Value::AssocArray(std::collections::HashMap::new())
    }

    /// Create a none value.
    pub fn none() -> Self {
        Value::None
    }

    /// Check if this value is none/unset.
    pub fn is_none(&self) -> bool {
        matches!(self, Value::None)
    }

    /// Check if this value is a string.
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Check if this value is an integer.
    pub fn is_integer(&self) -> bool {
        matches!(self, Value::Integer(_))
    }

    /// Check if this value is a float.
    pub fn is_float(&self) -> bool {
        matches!(self, Value::Float(_))
    }

    /// Check if this value is a number (integer or float).
    pub fn is_number(&self) -> bool {
        self.is_integer() || self.is_float()
    }

    /// Check if this value is an array.
    pub fn is_array(&self) -> bool {
        matches!(self, Value::Array(_))
    }

    /// Check if this value is an associative array.
    pub fn is_assoc_array(&self) -> bool {
        matches!(self, Value::AssocArray(_))
    }

    /// Get the value as a string.
    pub fn as_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Integer(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Boolean(b) => if *b { "1" } else { "0" }.to_string(),
            Value::Array(arr) => {
                let parts: Vec<String> = arr.iter().map(|v| v.as_string()).collect();
                parts.join(" ")
            }
            Value::AssocArray(map) => {
                let parts: Vec<String> = map
                    .iter()
                    .map(|(k, v)| format!("{}={}", k, v.as_string()))
                    .collect();
                parts.join(" ")
            }
            Value::None => String::new(),
        }
    }

    /// Get the value as an integer.
    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Value::Integer(n) => Some(*n),
            Value::Float(f) => Some(*f as i64),
            Value::String(s) => s.parse().ok(),
            Value::Boolean(b) => Some(if *b { 1 } else { 0 }),
            _ => None,
        }
    }

    /// Get the value as a float.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Integer(n) => Some(*n as f64),
            Value::String(s) => s.parse().ok(),
            _ => None,
        }
    }

    /// Get the value as a boolean.
    pub fn as_boolean(&self) -> bool {
        match self {
            Value::Boolean(b) => *b,
            Value::Integer(n) => *n != 0,
            Value::Float(f) => *f != 0.0,
            Value::String(s) => !s.is_empty() && s != "0" && s != "false" && s != "no",
            Value::Array(arr) => !arr.is_empty(),
            Value::AssocArray(map) => !map.is_empty(),
            Value::None => false,
        }
    }

    /// Get the value as an array.
    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    /// Get the length of this value.
    pub fn len(&self) -> usize {
        match self {
            Value::String(s) => s.len(),
            Value::Array(arr) => arr.len(),
            Value::AssocArray(map) => map.len(),
            _ => 0,
        }
    }

    /// Check if this value is empty.
    pub fn is_empty(&self) -> bool {
        match self {
            Value::String(s) => s.is_empty(),
            Value::Array(arr) => arr.is_empty(),
            Value::AssocArray(map) => map.is_empty(),
            Value::None => true,
            _ => false,
        }
    }

    /// Get the type name of this value.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::String(_) => "string",
            Value::Integer(_) => "integer",
            Value::Float(_) => "float",
            Value::Array(_) => "array",
            Value::AssocArray(_) => "assoc_array",
            Value::Boolean(_) => "boolean",
            Value::None => "none",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Integer(n)
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Integer(n as i64)
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Float(f)
    }
}

impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Boolean(b)
    }
}

impl From<Vec<Value>> for Value {
    fn from(v: Vec<Value>) -> Self {
        Value::Array(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_string() {
        let v = Value::string("hello");
        assert!(v.is_string());
        assert_eq!(v.as_string(), "hello");
    }

    #[test]
    fn test_value_integer() {
        let v = Value::integer(42);
        assert!(v.is_integer());
        assert_eq!(v.as_integer(), Some(42));
        assert_eq!(v.as_string(), "42");
    }

    #[test]
    fn test_value_float() {
        let v = Value::float(3.14);
        assert!(v.is_float());
        assert_eq!(v.as_float(), Some(3.14));
    }

    #[test]
    fn test_value_boolean() {
        let v = Value::boolean(true);
        assert!(v.as_boolean());
        assert_eq!(v.as_string(), "1");

        let v = Value::boolean(false);
        assert!(!v.as_boolean());
        assert_eq!(v.as_string(), "0");
    }

    #[test]
    fn test_value_array() {
        let v = Value::array_from(vec![
            Value::string("a"),
            Value::string("b"),
            Value::string("c"),
        ]);
        assert!(v.is_array());
        assert_eq!(v.len(), 3);
        assert_eq!(v.as_string(), "a b c");
    }

    #[test]
    fn test_value_none() {
        let v = Value::none();
        assert!(v.is_none());
        assert!(v.is_empty());
        assert_eq!(v.as_string(), "");
        assert!(!v.as_boolean());
    }

    #[test]
    fn test_value_type_name() {
        assert_eq!(Value::string("hello").type_name(), "string");
        assert_eq!(Value::integer(42).type_name(), "integer");
        assert_eq!(Value::float(3.14).type_name(), "float");
        assert_eq!(Value::boolean(true).type_name(), "boolean");
        assert_eq!(Value::array().type_name(), "array");
        assert_eq!(Value::none().type_name(), "none");
    }

    #[test]
    fn test_value_from_conversions() {
        let v: Value = "hello".into();
        assert_eq!(v, Value::String("hello".to_string()));

        let v: Value = 42i64.into();
        assert_eq!(v, Value::Integer(42));

        let v: Value = true.into();
        assert_eq!(v, Value::Boolean(true));
    }

    #[test]
    fn test_value_as_boolean() {
        assert!(!Value::string("").as_boolean());
        assert!(Value::string("yes").as_boolean());
        assert!(!Value::string("0").as_boolean());
        assert!(Value::string("1").as_boolean());
        assert!(!Value::string("false").as_boolean());
        assert!(Value::string("true").as_boolean());
    }
}
