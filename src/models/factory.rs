use diesel::prelude::*;
#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schemas::factories)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Factory {
    pub id: i32,
    pub name: String,
    pub address: String,
    pub fee: i32,
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
