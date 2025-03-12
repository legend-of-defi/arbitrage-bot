use std::str::FromStr;

use alloy::primitives::Address;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::pg::Pg;
use diesel::result::Error;
use diesel::serialize::ToSql;
use diesel::sql_types::Text;
use diesel::Queryable;
use diesel::Selectable;
use diesel::{ExpressionMethods, Insertable};
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;

/// The status of a factory
#[derive(Debug, Clone, Copy, PartialEq, Eq, AsExpression, FromSqlRow)]
#[diesel(sql_type = crate::schemas::sql_types::FactoryStatus)]
pub enum FactoryStatus {
    /// The factory is not fully synced yet - workers are still working on it
    Unsynced,
    /// The factory is syncing at this moment
    Syncing,
    /// The factory is fully synced - workers have finished working on it
    Synced,
    /// The factory is broken - workers have failed to sync it due to reverts
    Broken,
}

impl FromStr for FactoryStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Unsynced" => Ok(FactoryStatus::Unsynced),
            "Syncing" => Ok(FactoryStatus::Syncing),
            "Synced" => Ok(FactoryStatus::Synced),
            "Broken" => Ok(FactoryStatus::Broken),
            _ => Err("Invalid factory status".to_string()),
        }
    }
}

impl ToSql<crate::schemas::sql_types::FactoryStatus, diesel::pg::Pg> for FactoryStatus {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
    ) -> diesel::serialize::Result {
        let s = match self {
            FactoryStatus::Unsynced => "Unsynced",
            FactoryStatus::Syncing => "Syncing",
            FactoryStatus::Synced => "Synced",
            FactoryStatus::Broken => "Broken",
        };
        <str as ToSql<diesel::sql_types::Text, diesel::pg::Pg>>::to_sql(s, out)
    }
}

impl FromSql<crate::schemas::sql_types::FactoryStatus, Pg> for FactoryStatus {
    fn from_sql(
        bytes: <Pg as diesel::backend::Backend>::RawValue<'_>,
    ) -> deserialize::Result<Self> {
        let s = <String as FromSql<Text, Pg>>::from_sql(bytes)?;
        match FactoryStatus::from_str(&s) {
            Ok(status) => Ok(status),
            Err(e) => Err(Box::new(Error::DeserializationError(e.into()))
                as Box<dyn std::error::Error + Send + Sync>),
        }
    }
}

use crate::schemas::factories;

use super::pair::DBAddress;

/// A Uniswap V2 factory
#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schemas::factories)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Factory {
    /// The ID of the factory
    id: i32,
    /// The address of the factory
    address: DBAddress,
    /// The last pair ID of the factory
    last_pair_id: i32,
    /// The status of the factory
    ///
    /// This is future functionality.
    #[allow(dead_code)]
    status: FactoryStatus,
}

impl Factory {
    /// Get the ID of the factory
    #[must_use]
    pub fn id(&self) -> i32 {
        self.id
    }

    /// Get the address of the factory
    #[must_use]
    pub fn address(&self) -> Address {
        self.address.value
    }

    /// Get the last pair ID of the factory
    #[must_use]
    pub fn last_pair_id(&self) -> i32 {
        self.last_pair_id
    }

    /// Update the status of the factory
    ///
    /// # Errors
    ///
    /// Returns an error if the database update fails
    pub async fn update_status(
        &mut self,
        conn: &mut AsyncPgConnection,
        status: FactoryStatus,
    ) -> Result<(), Error> {
        diesel::update(factories::table)
            .filter(factories::id.eq(self.id()))
            .set(factories::status.eq(status))
            .execute(conn)
            .await?;

        Ok(())
    }
}

/// A new factory
#[derive(Insertable, Clone, Debug)]
#[diesel(table_name = crate::schemas::factories)]
pub struct NewFactory {
    /// The address of the factory
    address: DBAddress,
    /// The last pair ID of the factory
    last_pair_id: Option<i32>,
}

impl NewFactory {
    /// Create a new factory
    #[must_use]
    pub fn new(address: Address) -> Self {
        Self {
            address: DBAddress::new(address),
            last_pair_id: None,
        }
    }

    /// The address of the factory
    #[must_use]
    pub fn address(&self) -> Address {
        self.address.value
    }

    /// The last pair ID of the factory
    #[must_use]
    pub fn last_pair_id(&self) -> Option<i32> {
        self.last_pair_id
    }
}
