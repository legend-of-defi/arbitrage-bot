// @generated automatically by Diesel CLI.

/// A module containing custom SQL type definitions
///
/// (Automatically generated by Diesel.)
pub mod sql_types {
    /// The `factory_status` SQL type
    ///
    /// (Automatically generated by Diesel.)
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "factory_status"))]
    pub struct FactoryStatus;

    /// The `price_support_status` SQL type
    ///
    /// (Automatically generated by Diesel.)
    #[derive(diesel::query_builder::QueryId, Clone, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "price_support_status"))]
    pub struct PriceSupportStatus;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::FactoryStatus;

    /// Representation of the `factories` table.
    ///
    /// (Automatically generated by Diesel.)
    factories (id) {
        /// The `id` column of the `factories` table.
        ///
        /// Its SQL type is `Int4`.
        ///
        /// (Automatically generated by Diesel.)
        id -> Int4,
        /// The `address` column of the `factories` table.
        ///
        /// Its SQL type is `Varchar`.
        ///
        /// (Automatically generated by Diesel.)
        address -> Varchar,
        /// The `last_pair_id` column of the `factories` table.
        ///
        /// Its SQL type is `Int4`.
        ///
        /// (Automatically generated by Diesel.)
        last_pair_id -> Int4,
        /// The `status` column of the `factories` table.
        ///
        /// Its SQL type is `FactoryStatus`.
        ///
        /// (Automatically generated by Diesel.)
        status -> FactoryStatus,
    }
}

diesel::table! {
    /// Representation of the `pairs` table.
    ///
    /// (Automatically generated by Diesel.)
    pairs (id) {
        /// The `id` column of the `pairs` table.
        ///
        /// Its SQL type is `Int4`.
        ///
        /// (Automatically generated by Diesel.)
        id -> Int4,
        /// The `address` column of the `pairs` table.
        ///
        /// Its SQL type is `Varchar`.
        ///
        /// (Automatically generated by Diesel.)
        address -> Varchar,
        /// The `token0_id` column of the `pairs` table.
        ///
        /// Its SQL type is `Nullable<Int4>`.
        ///
        /// (Automatically generated by Diesel.)
        token0_id -> Nullable<Int4>,
        /// The `token1_id` column of the `pairs` table.
        ///
        /// Its SQL type is `Nullable<Int4>`.
        ///
        /// (Automatically generated by Diesel.)
        token1_id -> Nullable<Int4>,
        /// The `factory_id` column of the `pairs` table.
        ///
        /// Its SQL type is `Nullable<Int4>`.
        ///
        /// (Automatically generated by Diesel.)
        factory_id -> Nullable<Int4>,
        /// The `reserve0` column of the `pairs` table.
        ///
        /// Its SQL type is `Nullable<Numeric>`.
        ///
        /// (Automatically generated by Diesel.)
        reserve0 -> Nullable<Numeric>,
        /// The `reserve1` column of the `pairs` table.
        ///
        /// Its SQL type is `Nullable<Numeric>`.
        ///
        /// (Automatically generated by Diesel.)
        reserve1 -> Nullable<Numeric>,
        /// The `usd` column of the `pairs` table.
        ///
        /// Its SQL type is `Nullable<Int4>`.
        ///
        /// (Automatically generated by Diesel.)
        usd -> Nullable<Int4>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::PriceSupportStatus;

    /// Representation of the `tokens` table.
    ///
    /// (Automatically generated by Diesel.)
    tokens (id) {
        /// The `id` column of the `tokens` table.
        ///
        /// Its SQL type is `Int4`.
        ///
        /// (Automatically generated by Diesel.)
        id -> Int4,
        /// The `address` column of the `tokens` table.
        ///
        /// Its SQL type is `Varchar`.
        ///
        /// (Automatically generated by Diesel.)
        address -> Varchar,
        /// The `symbol` column of the `tokens` table.
        ///
        /// Its SQL type is `Nullable<Varchar>`.
        ///
        /// (Automatically generated by Diesel.)
        symbol -> Nullable<Varchar>,
        /// The `name` column of the `tokens` table.
        ///
        /// Its SQL type is `Nullable<Varchar>`.
        ///
        /// (Automatically generated by Diesel.)
        name -> Nullable<Varchar>,
        /// The `decimals` column of the `tokens` table.
        ///
        /// Its SQL type is `Nullable<Int4>`.
        ///
        /// (Automatically generated by Diesel.)
        decimals -> Nullable<Int4>,
        /// Exchange rate of the token in USD
        exchange_rate -> Nullable<Numeric>,
        /// Timestamp of when the exchange rate was last updated
        updated_last -> Nullable<Timestamp>,
        /// Indicates whether price data is available for this token from external APIs. NULL means not yet checked.
        price_support_status -> Nullable<PriceSupportStatus>,
    }
}

diesel::joinable!(pairs -> factories (factory_id));

diesel::allow_tables_to_appear_in_same_query!(factories, pairs, tokens,);
