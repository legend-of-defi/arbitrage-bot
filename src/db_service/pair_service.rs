#![allow(dead_code)]
use crate::arb::pool::{Pool, PoolId};
use crate::arb::token::TokenId;
use crate::models::pair::{NewPair, Pair};
use crate::models::token::Token;
use crate::schemas::{factories, pairs};
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::sql_types::{Integer, Nullable, Text};
use diesel::QueryableByName;
use std::collections::HashSet;
use diesel::dsl::count_star;

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
        address: &str,
        token0_id: i32,
        token1_id: i32,
        factory_id: i32,
    ) -> Pair {
        let new_pair = NewPair {
            address: address.to_string(),
            token0_id,
            token1_id,
            factory_id,
        };

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
    pub fn read_all_pairs(conn: &mut PgConnection) -> Vec<Pair> {
        pairs::table
            .select(Pair::as_select())
            .load(conn)
            .expect("Error loading pairs")
    }

    // Get pair with associated tokens
    pub fn read_pair_with_tokens(conn: &mut PgConnection, id: i32) -> Option<(Pair, Token, Token)> {
        use crate::schemas::tokens;

        let pair = pairs::table.find(id).first::<Pair>(conn).ok()?;

        let token0 = tokens::table
            .find(pair.token0_id)
            .first::<Token>(conn)
            .ok()?;

        let token1 = tokens::table
            .find(pair.token1_id)
            .first::<Token>(conn)
            .ok()?;

        Some((pair, token0, token1))
    }

    /// Loads all pools from the database with their associated tokens and factory information
    ///
    /// # Returns
    /// A `HashSet` of `Pool` objects representing all pairs in the database
    ///
    /// # Panics
    /// * If the SQL query fails
    /// * If the database connection fails
    /// * If data conversion between database and application types fails
    pub fn load_all_pools(conn: &mut PgConnection) -> HashSet<Pool> {
        // Use a raw SQL query with explicit joins
        let rows = diesel::sql_query(r"
            SELECT
                p.id, p.address, p.token0_id, p.token1_id, p.factory_id,
                t0.address as token0_address, t0.symbol as token0_symbol, t0.name as token0_name, t0.decimals as token0_decimals,
                t1.address as token1_address, t1.symbol as token1_symbol, t1.name as token1_name, t1.decimals as token1_decimals,
                f.address as factory_address, f.name as factory_name, f.fee as factory_fee, f.version as factory_version
            FROM pairs p
            INNER JOIN tokens t0 ON p.token0_id = t0.id
            INNER JOIN tokens t1 ON p.token1_id = t1.id
            INNER JOIN factories f ON p.factory_id = f.id
        ")
        .load::<PairWithTokens>(conn)
        .expect("Error loading pairs with tokens");

        // Create pools from the joined results
        let mut pools = HashSet::with_capacity(rows.len());

        for row in rows {
            pools.insert(Pool::new(
                PoolId::try_from(&*row.address).unwrap(),
                TokenId::try_from(&*row.token0_address).unwrap(),
                TokenId::try_from(&*row.token1_address).unwrap(),
                None,
                None,
            ));
        }

        pools
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
        address: &str,
        token0_id: i32,
        token1_id: i32,
        factory_id: i32,
    ) -> Result<Pair, diesel::result::Error> {
        pairs::table
            .filter(pairs::address.eq(address))
            .first(conn)
            .or_else(|_| {
                let new_pair = NewPair {
                    address: address.to_string(),
                    token0_id,
                    token1_id,
                    factory_id,
                };
                diesel::insert_into(pairs::table)
                    .values(&new_pair)
                    .returning(Pair::as_returning())
                    .get_result(conn)
            })
    }

    /// Get all pair addresses for a specific factory
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `factory_address` - The factory address as a string
    ///
    /// # Returns
    /// Vector of pair addresses as strings
    pub fn get_pair_addresses_by_factory(
        conn: &mut PgConnection,
        factory_address: String
    ) -> Result<Vec<String>, eyre::Report> {
        use crate::schemas::{factories, pairs};
        use diesel::prelude::*;

        let addresses = pairs::table
            .inner_join(factories::table)
            .filter(factories::address.eq(factory_address))
            .select(pairs::address)
            .load::<String>(conn)?;

        Ok(addresses)
    }

    pub fn count_pairs_by_factory_address(
        conn: &mut PgConnection,
        factory_address: &str
    ) -> Result<i64, diesel::result::Error> {
        pairs::table
            .inner_join(factories::table)
            .filter(factories::address.eq(factory_address))
            .select(count_star())
            .first(conn)
    }
}
