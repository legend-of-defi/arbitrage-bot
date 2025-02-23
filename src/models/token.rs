use diesel::{Queryable, Selectable, Insertable};

#[derive(Queryable, Selectable, Debug)]

#[diesel(table_name = crate::schemas::tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Token {
    #[allow(dead_code)]
    pub id: i32,
    #[allow(dead_code)]
    pub address: String,
    #[allow(dead_code)]
    pub symbol: Option<String>,
    #[allow(dead_code)]
    pub name: Option<String>,
    #[allow(dead_code)]
    pub decimals: i32,
}

#[derive(Insertable, Clone, Debug)]
#[diesel(table_name = crate::schemas::tokens)]
pub struct NewToken {
    pub address: String,
    pub symbol: Option<String>,
    pub name: Option<String>,
    pub decimals: i32,
}

