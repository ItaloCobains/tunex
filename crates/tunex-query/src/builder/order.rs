#[derive(Debug, Clone)]
pub enum OrderDir {
    Asc,
    Desc,
}

#[derive(Debug, Clone)]
pub struct OrderBy {
    pub column: String,
    pub direction: OrderDir,
}
