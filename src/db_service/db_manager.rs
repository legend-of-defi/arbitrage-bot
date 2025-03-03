#![allow(dead_code)]
use crate::db_service::{FactoryService, PairService, TokenService};
use crate::models::{
    factory::{Factory, NewFactory},
    pair::{NewPair, Pair},
    token::{NewToken, Token},
};
use crate::schemas::{factories, pairs, tokens};
use diesel::prelude::*;

pub struct DbManager {}

impl DbManager {
    /// Save or update complete DEX information
    /// This function handles the entire workflow of saving/updating factory, tokens, and pair information
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `factory_info` - Factory information
    /// * `token0_info` - First token information
    /// * `token1_info` - Second token information
    /// * `pair_address` - Pair contract address
    ///
    /// # Returns
    /// Tuple containing the saved/updated factory, tokens, and pair
    ///
    /// # Errors
    /// * If database transaction fails
    /// * If factory/token/pair operations fail
    /// * If database constraints are violated
    pub fn save_dex_info(
        conn: &mut PgConnection,
        factory_info: &NewFactory,
        token0_info: &NewToken,
        token1_info: &NewToken,
        pair_address: &str,
    ) -> Result<(Factory, Token, Token, Pair), diesel::result::Error> {
        conn.transaction(|conn| {
            let factory = FactoryService::read_or_create(
                conn,
                &factory_info.address,
                &factory_info.name,
                factory_info.fee,
                &factory_info.version,
            )?;

            let token0 = TokenService::read_or_create(
                conn,
                &token0_info.address,
                token0_info.symbol.as_deref(),
                token0_info.name.as_deref(),
                token0_info.decimals,
            )?;

            let token1 = TokenService::read_or_create(
                conn,
                &token1_info.address,
                token1_info.symbol.as_deref(),
                token1_info.name.as_deref(),
                token1_info.decimals,
            )?;

            let pair =
                PairService::read_or_create(conn, pair_address, token0.id, token1.id, factory.id)?;

            Ok((factory, token0, token1, pair))
        })
    }

    /// Batch save multiple DEX pairs
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `dex_infos` - Vector of tuples containing factory, tokens, and pair information
    ///
    /// # Returns
    /// Vector of saved/updated factory, tokens, and pair records
    ///
    /// # Errors
    /// * If any individual save operation fails
    /// * If database transaction fails
    /// * If database constraints are violated
    pub fn batch_save_dex_info(
        conn: &mut PgConnection,
        dex_infos: Vec<(NewFactory, NewToken, NewToken, String)>,
    ) -> Vec<(Factory, Token, Token, Pair)> {
        // Execute everything in a single transaction
        conn.transaction(|conn| {
            let mut results = Vec::with_capacity(dex_infos.len());

            for (factory, token0, token1, pair_address) in dex_infos {
                // Reuse the existing save_dex_info logic, but without the transaction wrapper
                // since we're already in a transaction

                let factory_result = FactoryService::read_or_create(
                    conn,
                    &factory.address,
                    &factory.name,
                    factory.fee,
                    &factory.version,
                );

                if let Err(e) = factory_result {
                    println!("Error saving factory: {:?}", e);
                    continue;
                }
                let factory_record = factory_result.unwrap();

                let token0_result = TokenService::read_or_create(
                    conn,
                    &token0.address,
                    token0.symbol.as_deref(),
                    token0.name.as_deref(),
                    token0.decimals,
                );

                if let Err(e) = token0_result {
                    println!("Error saving token0: {:?}", e);
                    continue;
                }
                let token0_record = token0_result.unwrap();

                let token1_result = TokenService::read_or_create(
                    conn,
                    &token1.address,
                    token1.symbol.as_deref(),
                    token1.name.as_deref(),
                    token1.decimals,
                );

                if let Err(e) = token1_result {
                    println!("Error saving token1: {:?}", e);
                    continue;
                }
                let token1_record = token1_result.unwrap();

                let pair_result = PairService::read_or_create(
                    conn,
                    &pair_address,
                    token0_record.id,
                    token1_record.id,
                    factory_record.id
                );

                if let Err(e) = pair_result {
                    println!("Error saving pair: {:?}", e);
                    continue;
                }
                let pair_record = pair_result.unwrap();

                results.push((factory_record, token0_record, token1_record, pair_record));
            }

            Ok(results)
        }).unwrap_or_default()
    }

    // Helper functions
    fn read_or_create_factory(
        conn: &mut PgConnection,
        info: NewFactory,
    ) -> Result<Factory, diesel::result::Error> {
        factories::table
            .filter(factories::address.eq(info.address.clone()))
            .first(conn)
            .or_else(|_| {
                let new_factory = info;
                diesel::insert_into(factories::table)
                    .values(&new_factory)
                    .returning(Factory::as_returning())
                    .get_result(conn)
            })
    }

    fn read_or_create_token(
        conn: &mut PgConnection,
        info: NewToken,
    ) -> Result<Token, diesel::result::Error> {
        if let Ok(mut token) = tokens::table
            .filter(tokens::address.eq(info.address.clone()))
            .first::<Token>(conn)
        {
            // Update token info if new data is available
            if info.symbol.is_some() || info.name.is_some() {
                token = diesel::update(tokens::table.find(token.id))
                    .set((tokens::symbol.eq(info.symbol), tokens::name.eq(info.name)))
                    .returning(Token::as_returning())
                    .get_result(conn)?;
            }
            Ok(token)
        } else {
            let new_token = info;
            diesel::insert_into(tokens::table)
                .values(&new_token)
                .returning(Token::as_returning())
                .get_result(conn)
        }
    }

    fn read_or_create_pair(
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
                    address: String::from(address),
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

    /// Gets the last pair index for a given factory
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `factory_addr` - Factory contract address
    ///
    /// # Errors
    /// * If database query fails
    pub fn get_last_pair_index(
        conn: &mut PgConnection,
        factory_addr: &str,
    ) -> Result<Option<i32>, eyre::Report> {
        use diesel::dsl::max;

        pairs::table
            .inner_join(factories::table)
            .filter(factories::address.eq(factory_addr))
            .select(max(pairs::id))
            .first::<Option<i32>>(conn)
            .map_err(|e| eyre::eyre!(e))
    }
}
