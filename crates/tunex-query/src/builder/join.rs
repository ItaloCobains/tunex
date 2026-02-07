#[derive(Debug, Clone)]
pub enum JoinKind {
    Inner,
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub struct Join {
    pub kind: JoinKind,
    pub table: String,
    pub on_left: String,
    pub on_right: String,
}
