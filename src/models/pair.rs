use diesel::{Insertable, Queryable, Selectable};

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schemas::pairs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Pair {
    pub id: i32,
    pub address: String,
    pub token0_id: i32,
    pub token1_id: i32,
    #[allow(dead_code)]
    pub factory_id: i32,
}

#[derive(Insertable)]
#[diesel(table_name = crate::schemas::pairs)]
pub struct NewPair {
    pub address: String,
    pub token0_id: i32,
    pub token1_id: i32,
    pub factory_id: i32,
}

