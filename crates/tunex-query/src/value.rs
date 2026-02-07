use chrono::NaiveDateTime;
use sqlx::postgres::PgArguments;
use sqlx::Arguments;

#[derive(Debug, Clone)]
pub enum Value {
    Int(i32),
    BigInt(i64),
    Text(String),
    Bool(bool),
    Timestamp(NaiveDateTime),
    Null,
}

impl Value {
    pub(crate) fn bind_to(&self, args: &mut PgArguments) {
        match self {
            Value::Int(v) => args.add(v).unwrap(),
            Value::BigInt(v) => args.add(v).unwrap(),
            Value::Text(v) => args.add(v).unwrap(),
            Value::Bool(v) => args.add(v).unwrap(),
            Value::Timestamp(v) => args.add(v).unwrap(),
            Value::Null => args.add(None::<String>).unwrap(),
        }
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Int(v)
    }
}

impl From<i64> for Value {
    fn from(v: i64) -> Self {
        Value::BigInt(v)
    }
}

impl From<&str> for Value {
    fn from(v: &str) -> Self {
        Value::Text(v.to_string())
    }
}

impl From<String> for Value {
    fn from(v: String) -> Self {
        Value::Text(v)
    }
}

impl From<bool> for Value {
    fn from(v: bool) -> Self {
        Value::Bool(v)
    }
}

impl From<NaiveDateTime> for Value {
    fn from(v: NaiveDateTime) -> Self {
        Value::Timestamp(v)
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(v: Option<T>) -> Self {
        match v {
            Some(val) => val.into(),
            None => Value::Null,
        }
    }
}
