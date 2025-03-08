#![allow(dead_code)]

use crate::models::pair::{NewPair, Pair};
use crate::models::token::Token;
use crate::schemas::pairs;
use alloy::primitives::Address;
use bigdecimal::BigDecimal;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::sql_types::{Integer, Nullable, Numeric, Text};
use diesel::QueryableByName;
use eyre::Result;

pub struct PairService;

#[derive(QueryableByName, Debug)]
pub struct PairWithTokens {
    #[diesel(sql_type = Integer)]
    pub id: i32,
    #[diesel(sql_type = Text)]
    pub address: String,
    #[diesel(sql_type = Integer)]
    pub token0_id: i32,
    #[diesel(sql_type = Integer)]
    pub token1_id: i32,
    #[diesel(sql_type = Integer)]
    pub factory_id: i32,
    #[diesel(sql_type = Numeric)]
    pub reserve0: BigDecimal,
    #[diesel(sql_type = Numeric)]
    pub reserve1: BigDecimal,
    #[diesel(sql_type = Integer)]
    pub usd: i32,

    #[diesel(sql_type = Text)]
    pub token0_address: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub token0_symbol: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub token0_name: Option<String>,
    #[diesel(sql_type = Integer)]
    pub token0_decimals: i32,

    #[diesel(sql_type = Text)]
    pub token1_address: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub token1_symbol: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub token1_name: Option<String>,
    #[diesel(sql_type = Integer)]
    pub token1_decimals: i32,

    #[diesel(sql_type = Text)]
    pub factory_address: String,
    #[diesel(sql_type = Text)]
    pub factory_name: String,
    #[diesel(sql_type = Integer)]
    pub factory_fee: i32,
    #[diesel(sql_type = Text)]
    pub factory_version: String,
}

impl PairService {
    /// Create a new pair in the database
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `address` - Pair contract address
    /// * `token0_id` - ID of the first token
    /// * `token1_id` - ID of the second token
    /// * `factory_id` - ID of the factory
    ///
    /// # Returns
    /// The created pair record
    ///
    /// # Panics
    /// * If database insertion fails
    /// * If pair creation violates constraints
    pub fn create_pair(
        conn: &mut PgConnection,
        address: Address,
        token0_id: i32,
        token1_id: i32,
        factory_id: i32,
    ) -> Pair {
        let new_pair = NewPair::new(address, token0_id, token1_id, factory_id);

        diesel::insert_into(pairs::table)
            .values(&new_pair)
            .returning(Pair::as_returning())
            .get_result(conn)
            .expect("Error saving new pair")
    }

    /// Create a new pair in the database with reserve and USD values
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `address` - Pair contract address
    /// * `token0_id` - ID of the first token
    /// * `token1_id` - ID of the second token
    /// * `factory_id` - ID of the factory
    /// * `reserve0` - Reserve of token0
    /// * `reserve1` - Reserve of token1
    /// * `usd` - USD value of the pair
    ///
    /// # Returns
    /// The created pair record
    ///
    /// # Panics
    /// * If database insertion fails
    /// * If pair creation violates constraints
    #[allow(clippy::too_many_arguments)]
    pub fn create_pair_with_reserves(
        conn: &mut PgConnection,
        address: Address,
        token0_id: i32,
        token1_id: i32,
        factory_id: i32,
        reserve0: BigDecimal,
        reserve1: BigDecimal,
        usd: i32,
    ) -> Pair {
        let new_pair = NewPair::new_with_reserves(
            address, token0_id, token1_id, factory_id, reserve0, reserve1, usd,
        );

        diesel::insert_into(pairs::table)
            .values(&new_pair)
            .returning(Pair::as_returning())
            .get_result(conn)
            .expect("Error saving new pair")
    }

    // Read
    pub fn read_pair(conn: &mut PgConnection, id: i32) -> Option<Pair> {
        pairs::table
            .find(id)
            .select(Pair::as_select())
            .first(conn)
            .ok()
    }

    pub fn read_pair_by_address(conn: &mut PgConnection, address: &str) -> Option<Pair> {
        pairs::table
            .filter(pairs::address.eq(address))
            .select(Pair::as_select())
            .first(conn)
            .ok()
    }

    /// Get all pairs for a specific factory
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `id` - Factory ID
    ///
    /// # Returns
    /// Vector of pairs associated with the factory
    ///
    /// # Panics
    /// * If database query fails
    /// * If pairs cannot be loaded
    pub fn read_pairs_by_factory(conn: &mut PgConnection, id: i32) -> Vec<Pair> {
        pairs::table
            .filter(pairs::factory_id.eq(id))
            .select(Pair::as_select())
            .load(conn)
            .expect("Error loading pairs")
    }

    /// Get all pairs from the database
    ///
    /// # Arguments
    /// * `conn` - Database connection
    ///
    /// # Returns
    /// Vector of all pair records
    ///
    /// # Panics
    /// * If database query fails
    /// * If pairs cannot be loaded
    // pub async fn read_all_pairs(pool: &DbPool) -> Result<Vec<Pair>> {
    //     let client = pool.get().await?;
    //     let rows = client.query("SELECT * FROM pairs", &[]).await?;
    //     let pairs = rows.into_iter().map(|row| {
    //         let address_str: String = row.get("address");
    //         let reserve0_str: String = row.get("reserve0");
    //         let reserve1_str: String = row.get("reserve1");
    //         Pair {
    //             id: row.get("id"),
    //             address: address_str.parse().unwrap(),
    //             token0_id: row.get("token0_id"),
    //             token1_id: row.get("token1_id"),
    //             factory_id: row.get("factory_id"),
    //             reserve0: reserve0_str.parse().unwrap(),
    //             reserve1: reserve1_str.parse().unwrap(),
    //             usd: row.get("usd"),
    //         }
    //     }).collect();
    //     Ok(pairs)
    // }
    // Get pair with associated tokens
    pub fn read_pair_with_tokens(conn: &mut PgConnection, id: i32) -> Option<(Pair, Token, Token)> {
        use crate::schemas::tokens;

        let pair = pairs::table
            .find(id)
            .select(Pair::as_select())
            .first::<Pair>(conn)
            .ok()?;

        let token0 = tokens::table
            .find(pair.token0_id.unwrap())
            .first::<Token>(conn)
            .ok()?;

        let token1 = tokens::table
            .find(pair.token1_id.unwrap())
            .first::<Token>(conn)
            .ok()?;

        Some((pair, token0, token1))
    }

    /// Get all pairs that include a specific token
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `id` - Token ID
    ///
    /// # Returns
    /// Vector of pairs containing the specified token
    ///
    /// # Panics
    /// * If database query fails
    /// * If pairs cannot be loaded
    pub fn read_pairs_by_token(conn: &mut PgConnection, id: i32) -> Vec<Pair> {
        pairs::table
            .filter(pairs::token0_id.eq(id).or(pairs::token1_id.eq(id)))
            .select(Pair::as_select())
            .load(conn)
            .expect("Error loading pairs by token")
    }

    // Delete
    pub fn delete_pair(conn: &mut PgConnection, id: i32) -> bool {
        diesel::delete(pairs::table.find(id)).execute(conn).is_ok()
    }

    /// Get or create a pair
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `address` - Pair contract address
    /// * `token0_id` - ID of the first token
    /// * `token1_id` - ID of the second token
    /// * `factory_id` - ID of the factory
    ///
    /// # Returns
    /// Result containing either the existing or newly created pair
    ///
    /// # Errors
    /// * If database operations fail
    /// * If pair creation violates constraints
    /// * If pair lookup fails
    pub fn read_or_create(
        conn: &mut PgConnection,
        address: Address,
        token0_id: i32,
        token1_id: i32,
        factory_id: i32,
    ) -> Result<Pair> {
        pairs::table
            .filter(pairs::address.eq(address.to_string()))
            .select(Pair::as_select())
            .first(conn)
            .or_else(|_| {
                let new_pair = NewPair::new(address, token0_id, token1_id, factory_id);
                diesel::insert_into(pairs::table)
                    .values(&new_pair)
                    .returning(Pair::as_returning())
                    .get_result(conn)
                    .map_err(|e| eyre::eyre!(e))
            })
    }

    /// Get or create a pair with reserve and USD values
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `address` - Pair contract address
    /// * `token0_id` - ID of the first token
    /// * `token1_id` - ID of the second token
    /// * `factory_id` - ID of the factory
    /// * `reserve0` - Reserve of token0
    /// * `reserve1` - Reserve of token1
    /// * `usd` - USD value of the pair
    ///
    /// # Returns
    /// Result containing either the existing or newly created pair
    ///
    /// # Errors
    /// * If database operations fail
    /// * If pair creation violates constraints
    /// * If pair lookup fails
    #[allow(clippy::too_many_arguments)]
    pub fn read_or_create_with_reserves(
        conn: &mut PgConnection,
        address: Address,
        token0_id: i32,
        token1_id: i32,
        factory_id: i32,
        reserve0: BigDecimal,
        reserve1: BigDecimal,
        usd: i32,
    ) -> Result<Pair> {
        pairs::table
            .filter(pairs::address.eq(address.to_string()))
            .select(Pair::as_select())
            .first(conn)
            .or_else(|_| {
                let new_pair = NewPair::new_with_reserves(
                    address, token0_id, token1_id, factory_id, reserve0, reserve1, usd,
                );
                diesel::insert_into(pairs::table)
                    .values(&new_pair)
                    .returning(Pair::as_returning())
                    .get_result(conn)
                    .map_err(|e| eyre::eyre!(e))
            })
    }

    pub fn update_pair_reserves(
        conn: &mut PgConnection,
        pair_id: i32,
        reserve0: BigDecimal,
        reserve1: BigDecimal,
        usd: i32,
    ) -> Result<Pair> {
        diesel::update(pairs::table.find(pair_id))
            .set((
                pairs::reserve0.eq(reserve0),
                pairs::reserve1.eq(reserve1),
                pairs::usd.eq(usd),
            ))
            .returning(Pair::as_returning())
            .get_result(conn)
            .map_err(|e| eyre::eyre!(e))
    }
}
