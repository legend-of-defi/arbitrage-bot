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

/// A Uniswap V2 pair
#[derive(Queryable, Selectable, Debug)]
#[diesel(table_name = crate::schemas::pairs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Pair {
    /// The ID of the pair
    pub id: i32,
    /// The address of the pair
    pub address: DBAddress,
    /// The FK of the token0 - tokens.id
    pub token0_id: Option<i32>,
    /// The FK of the token1 - tokens.id
    pub token1_id: Option<i32>,
    /// The FK of the factory - factories.id
    ///
    /// This is future functionality.
    #[allow(dead_code)]
    pub factory_id: Option<i32>,
    /// The reserve of the token0
    pub reserve0: Option<BigDecimal>,
    /// The reserve of the token1
    pub reserve1: Option<BigDecimal>,
    /// The USD value of the pair
    ///
    /// This is future functionality.
    #[allow(dead_code)]
    pub usd: Option<i32>,
}

impl Pair {
    /// The address of the pair
    #[must_use]
    pub fn address(&self) -> Address {
        self.address.value
    }

    /// The ID of the pair
    #[must_use]
    pub fn id(&self) -> i32 {
        self.id
    }
}

/// A database address type
/// Wrap Alloy's Address for strict typing
#[derive(Debug, FromSqlRow, AsExpression, Clone)]
#[diesel(sql_type = Text)]
pub struct DBAddress {
    /// The address
    pub value: Address,
}

impl DBAddress {
    /// Create a new database address
    #[must_use]
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

/// A new pair
#[derive(Insertable, Debug)]
#[diesel(table_name = crate::schemas::pairs)]
pub struct NewPair {
    /// The address of the pair
    pub address: DBAddress,
    /// The FK of the token0 - tokens.id
    pub token0_id: i32,
    /// The FK of the token1 - tokens.id
    pub token1_id: i32,
    /// The FK of the factory - factories.id
    pub factory_id: i32,
    /// The reserve of the token0
    pub reserve0: BigDecimal,
    /// The reserve of the token1
    pub reserve1: BigDecimal,
    /// The USD value of the pair
    pub usd: i32,
}

impl NewPair {
    /// Create a new pair
    #[must_use]
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

    /// Create a new pair with reserves
    #[must_use]
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

    /// The address of the pair
    #[must_use]
    pub fn address(&self) -> Address {
        self.address.value
    }

    /// The ID of the token0
    #[must_use]
    pub fn token0_id(&self) -> i32 {
        self.token0_id
    }

    /// The ID of the token1
    #[must_use]
    pub fn token1_id(&self) -> i32 {
        self.token1_id
    }

    /// The ID of the factory
    #[must_use]
    pub fn factory_id(&self) -> i32 {
        self.factory_id
    }

    /// The reserve of the token0
    #[must_use]
    pub fn reserve0(&self) -> &BigDecimal {
        &self.reserve0
    }

    /// The reserve of the token1
    #[must_use]
    pub fn reserve1(&self) -> &BigDecimal {
        &self.reserve1
    }

    /// The USD value of the pair
    #[must_use]
    pub fn usd(&self) -> i32 {
        self.usd
    }
}
