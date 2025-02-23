use diesel::prelude::*;
#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schemas::factories)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Factory {
    #[allow(dead_code)]
    pub id: i32,
    #[allow(dead_code)]
    pub name: String,
    #[allow(dead_code)]
    pub address: String,
    #[allow(dead_code)]
    pub fee: i32,
    #[allow(dead_code)]
    pub version: String,
}

#[derive(Insertable, Clone, Debug)]
#[diesel(table_name = crate::schemas::factories)]
pub struct NewFactory {
    pub name: String,
    pub address: String,
    pub fee: i32,
    pub version: String,
}

