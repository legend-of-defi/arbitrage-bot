use alloy::primitives::Address;
use bigdecimal::BigDecimal;
use diesel::deserialize::{self, FromSql, FromSqlRow};
use diesel::expression::AsExpression;
use diesel::pg::{Pg, PgValue};
use diesel::sql_types::Text;
use diesel::{
    serialize::{self, IsNull, Output, ToSql},
    Insertable, Queryable, Selectable,
};
use eyre::Error;
use std::io::Write;
use std::str::FromStr;

#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schemas::pairs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Pair {
    pub id: i32,
    pub address: DBAddress,
    pub token0_id: Option<i32>,
    pub token1_id: Option<i32>,
    pub factory_id: Option<i32>,
    pub reserve0: Option<BigDecimal>,
    pub reserve1: Option<BigDecimal>,
    pub usd: Option<i32>,
}

impl Pair {
    pub fn address(&self) -> Address {
        self.address.value
    }

    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn token0_id(&self) -> Option<i32> {
        self.token0_id
    }

    pub fn token1_id(&self) -> Option<i32> {
        self.token1_id
    }

    pub fn factory_id(&self) -> Option<i32> {
        self.factory_id
    }

    pub fn reserve0(&self) -> &Option<BigDecimal> {
        &self.reserve0
    }

    pub fn reserve1(&self) -> &Option<BigDecimal> {
        &self.reserve1
    }

    pub fn usd(&self) -> Option<i32> {
        self.usd
    }
}

#[derive(Debug, FromSqlRow, AsExpression, Clone)]
#[diesel(sql_type = Text)]
pub struct DBAddress {
    pub value: Address,
}

impl DBAddress {
    pub fn new(address: Address) -> Self {
        Self { value: address }
    }
}
impl FromStr for DBAddress {
    fn from_str(s: &str) -> Result<Self, Error> {
        let address = Address::parse_checksummed(s, None)?;
        Ok(Self { value: address })
    }

    type Err = Error;
}

#[derive(Insertable, Debug)]
#[diesel(table_name = crate::schemas::pairs)]
pub struct NewPair {
    pub address: DBAddress,
    pub token0_id: i32,
    pub token1_id: i32,
    pub factory_id: i32,
    pub reserve0: BigDecimal,
    pub reserve1: BigDecimal,
    pub usd: i32,
}

impl NewPair {
    pub fn new(address: Address, token0_id: i32, token1_id: i32, factory_id: i32) -> Self {
        Self {
            address: DBAddress::new(address),
            token0_id,
            token1_id,
            factory_id,
            reserve0: BigDecimal::from(0),
            reserve1: BigDecimal::from(0),
            usd: 0,
        }
    }

    pub fn new_with_reserves(
        address: Address,
        token0_id: i32,
        token1_id: i32,
        factory_id: i32,
        reserve0: BigDecimal,
        reserve1: BigDecimal,
        usd: i32,
    ) -> Self {
        Self {
            address: DBAddress::new(address),
            token0_id,
            token1_id,
            factory_id,
            reserve0,
            reserve1,
            usd,
        }
    }

    pub fn address(&self) -> Address {
        self.address.value
    }

    pub fn token0_id(&self) -> i32 {
        self.token0_id
    }

    pub fn token1_id(&self) -> i32 {
        self.token1_id
    }

    pub fn factory_id(&self) -> i32 {
        self.factory_id
    }

    pub fn reserve0(&self) -> &BigDecimal {
        &self.reserve0
    }

    pub fn reserve1(&self) -> &BigDecimal {
        &self.reserve1
    }

    pub fn usd(&self) -> i32 {
        self.usd
    }
}

impl ToSql<Text, diesel::pg::Pg> for DBAddress {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, diesel::pg::Pg>) -> serialize::Result {
        let address = format!("{}", self.value);
        out.write_all(address.as_bytes())?;
        Ok(IsNull::No)
    }
}

impl FromSql<Text, Pg> for DBAddress {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let bytes = bytes.as_bytes();
        let addr = Address::parse_checksummed(std::str::from_utf8(bytes)?, None)?;
        Ok(DBAddress { value: addr })
    }
}
