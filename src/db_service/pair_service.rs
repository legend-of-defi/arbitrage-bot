use crate::models::pair::{Pair, NewPair};
use crate::models::token::Token;
use crate::schemas::pairs;
use diesel::prelude::*;

pub struct PairService;

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
    #[allow(dead_code)]
    pub fn read_pair(conn: &mut PgConnection, id: i32) -> Option<Pair> {
        pairs::table
            .find(id)
            .select(Pair::as_select())
            .first(conn)
            .ok()
    }

    #[allow(dead_code)]
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
    #[allow(dead_code)]
    pub fn read_pair_with_tokens(conn: &mut PgConnection, id: i32) -> Option<(Pair, Token, Token)> {
        use crate::schemas::tokens;

        let pair = pairs::table
            .find(id)
            .first::<Pair>(conn)
            .ok()?;

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
            .filter(
                pairs::token0_id
                    .eq(id)
                    .or(pairs::token1_id.eq(id))
            )
            .select(Pair::as_select())
            .load(conn)
            .expect("Error loading pairs by token")
    }

    // Delete
    #[allow(dead_code)]
    pub fn delete_pair(conn: &mut PgConnection, id: i32) -> bool {
        diesel::delete(pairs::table.find(id))
            .execute(conn)
            .is_ok()
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
}
