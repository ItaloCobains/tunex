pub trait Migration: Send + Sync {
    fn version(&self) -> i64;
    fn name(&self) -> &str;
    fn up_sql(&self) -> &str;
    fn down_sql(&self) -> &str;
}
