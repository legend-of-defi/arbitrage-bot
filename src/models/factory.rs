use alloy::primitives::Address;
use diesel::prelude::*;

use super::pair::DBAddress;
#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schemas::factories)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Factory {
    id: i32,
    address: DBAddress,
    last_pair_id: Option<i32>,
}

impl Factory {
    pub fn new(id: i32, address: Address) -> Self {
        Self {
            id,
            address: DBAddress::new(address),
            last_pair_id: None,
        }
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn address(&self) -> Address {
        self.address.value
    }

    pub fn last_pair_id(&self) -> Option<i32> {
        self.last_pair_id
    }
}

#[derive(Insertable, Clone, Debug)]
#[diesel(table_name = crate::schemas::factories)]
pub struct NewFactory {
    address: DBAddress,
    last_pair_id: Option<i32>,
}

impl NewFactory {
    pub fn new(address: Address) -> Self {
        Self {
            address: DBAddress::new(address),
            last_pair_id: None,
        }
    }

    pub fn address(&self) -> Address {
        self.address.value
    }

    pub fn last_pair_id(&self) -> Option<i32> {
        self.last_pair_id
    }
}
