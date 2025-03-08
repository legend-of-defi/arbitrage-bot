// impl TokenService {
//     /// Create a new token in the database
//     ///
//     /// # Arguments
//     /// * `conn` - Database connection
//     /// * `address` - Token contract address
//     /// * `symbol` - Optional token symbol
//     /// * `name` - Optional token name
//     /// * `decimals` - Token decimals
//     ///
//     /// # Returns
//     /// The created token record
//     ///
//     /// # Panics
//     /// * If database insertion fails
//     /// * If token creation violates constraints
//     pub fn create_token(
//         conn: &mut PgConnection,
//         address: Address,
//         symbol: Option<&str>,
//         name: Option<&str>,
//         decimals: i32,
//     ) -> Token {
//         let new_token = NewToken::new(
//             address,
//             symbol.map(ToString::to_string),
//             name.map(ToString::to_string),
//             decimals,
//         );

//         diesel::insert_into(tokens::table)
//             .values(&new_token)
//             .returning(Token::as_returning())
//             .get_result(conn)
//             .expect("Error saving new token")
//     }

//     // Read
//     pub fn read_token(conn: &mut PgConnection, id: i32) -> Option<Token> {
//         tokens::table
//             .find(id)
//             .select(Token::as_select())
//             .first(conn)
//             .ok()
//     }

//     pub fn read_token_by_address(conn: &mut PgConnection, address: &str) -> Option<Token> {
//         tokens::table
//             .filter(tokens::address.eq(address))
//             .select(Token::as_select())
//             .first(conn)
//             .ok()
//     }

//     /// Get all tokens from the database
//     ///
//     /// # Arguments
//     /// * `conn` - Database connection
//     ///
//     /// # Returns
//     /// Vector of all token records
//     ///
//     /// # Panics
//     /// * If database query fails
//     /// * If token records cannot be loaded
//     pub fn read_all_tokens(conn: &mut PgConnection) -> Vec<Token> {
//         tokens::table
//             .select(Token::as_select())
//             .load(conn)
//             .expect("Error loading tokens")
//     }

//     /// Get tokens with their associated pairs count
//     ///
//     /// # Arguments
//     /// * `conn` - Database connection
//     ///
//     /// # Returns
//     /// Vector of tuples containing token and its pairs count
//     ///
//     /// # Panics
//     /// * If database query fails
//     /// * If join operation fails
//     pub fn read_tokens_with_pairs_count(conn: &mut PgConnection) -> Vec<(Token, i64)> {
//         use crate::schemas::pairs::dsl::{pairs, token0_id, token1_id};

//         tokens::table
//             .left_join(
//                 pairs.on(token0_id
//                     .eq(tokens::id.nullable())
//                     .or(token1_id.eq(tokens::id.nullable()))),
//             )
//             .group_by(tokens::all_columns)
//             .select((
//                 Token::as_select(),
//                 diesel::dsl::sql::<diesel::sql_types::BigInt>("COALESCE(COUNT(*), 0)"),
//             ))
//             .load(conn)
//             .expect("Error loading tokens with pairs count")
//     }

//     // Update token info
//     pub fn update_token_info(
//         conn: &mut PgConnection,
//         id: i32,
//         symbol: Option<&str>,
//         name: Option<&str>,
//     ) -> Option<Token> {
//         diesel::update(tokens::table.find(id))
//             .set((tokens::symbol.eq(symbol), tokens::name.eq(name)))
//             .returning(Token::as_returning())
//             .get_result(conn)
//             .ok()
//     }

//     /// Get or create token with optional update
//     ///
//     /// # Arguments
//     /// * `conn` - Database connection
//     /// * `address` - Token contract address
//     /// * `symbol` - Optional token symbol
//     /// * `name` - Optional token name
//     /// * `decimals` - Token decimals
//     ///
//     /// # Returns
//     /// Result containing either the existing/updated token or a new token
//     ///
//     /// # Errors
//     /// * If database operations fail
//     /// * If token creation violates constraints
//     pub fn read_or_create(
//         conn: &mut PgConnection,
//         address: Address,
//         symbol: Option<String>,
//         name: Option<String>,
//         decimals: i32,
//     ) -> Result<Token> {
//         if let Ok(mut token) = tokens::table
//             .filter(tokens::address.eq(address.to_string()))
//             .first::<Token>(conn)
//         {
//             if symbol.is_some() || name.is_some() {
//                 token = diesel::update(tokens::table.find(token.id()))
//                     .set((tokens::symbol.eq(symbol), tokens::name.eq(name)))
//                     .returning(Token::as_returning())
//                     .get_result(conn)?;
//             }
//             Ok(token)
//         } else {
//             let new_token = NewToken::new(address, symbol, name, decimals);
//             diesel::insert_into(tokens::table)
//                 .values(&new_token)
//                 .returning(Token::as_returning())
//                 .get_result(conn)
//                 .map_err(|e| eyre::eyre!(e))
//         }
//     }
// }
