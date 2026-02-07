use chrono::NaiveDateTime;
use sqlx::{postgres::PgRow, Row};

pub trait RowExt {
    fn get_i32(&self, col: &str) -> i32;
    fn get_i64(&self, col: &str) -> i64;
    fn get_string(&self, col: &str) -> String;
    fn get_bool(&self, col: &str) -> bool;
    fn get_opt_timestamp(&self, col: &str) -> Option<NaiveDateTime>;
}

impl RowExt for PgRow {
    fn get_i32(&self, col: &str) -> i32 {
        self.get(col)
    }
    fn get_i64(&self, col: &str) -> i64 {
        self.get(col)
    }
    fn get_string(&self, col: &str) -> String {
        self.get(col)
    }
    fn get_bool(&self, col: &str) -> bool {
        self.get(col)
    }
    fn get_opt_timestamp(&self, col: &str) -> Option<NaiveDateTime> {
        self.get(col)
    }
}
