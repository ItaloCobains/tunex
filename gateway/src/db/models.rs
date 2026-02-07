use tunex_query::chrono::NaiveDateTime;
use tunex_query::RowExt;

#[derive(Debug, Clone)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password: String,
    pub created_at: Option<NaiveDateTime>,
}

impl User {
    pub fn from_row(row: &tunex_query::sqlx::postgres::PgRow) -> Self {
        Self {
            id: row.get_i32("id"),
            name: row.get_string("name"),
            email: row.get_string("email"),
            password: row.get_string("password"),
            created_at: row.get_opt_timestamp("created_at"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Tunnel {
    pub id: i32,
    pub user_id: i32,
    pub ip: String,
    pub created_at: Option<NaiveDateTime>,
    pub updated_at: Option<NaiveDateTime>,
}

impl Tunnel {
    pub fn from_row(row: &tunex_query::sqlx::postgres::PgRow) -> Self {
        Self {
            id: row.get_i32("id"),
            user_id: row.get_i32("user_id"),
            ip: row.get_string("ip"),
            created_at: row.get_opt_timestamp("created_at"),
            updated_at: row.get_opt_timestamp("updated_at"),
        }
    }
}
