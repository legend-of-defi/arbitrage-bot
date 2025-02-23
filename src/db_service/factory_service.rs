use crate::models::factory::{Factory, NewFactory};
use crate::schemas::factories;
use diesel::prelude::*;

#[allow(dead_code)]
pub struct FactoryService;

impl FactoryService {
    /// Create a new factory in the database
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `name` - Factory name
    /// * `address` - Factory contract address
    /// * `fee` - Trading fee in basis points
    /// * `version` - Protocol version
    ///
    /// # Returns
    /// The created factory record
    ///
    /// # Panics
    /// * If database insertion fails
    /// * If factory creation violates constraints
    #[allow(dead_code)]
    pub fn create_factory(
        conn: &mut PgConnection,
        name: &str,
        address: &str,
        fee: i32,
        version: &str,
    ) -> Factory {
        let new_factory = NewFactory {
            name: name.to_string(),
            address: address.to_string(),
            fee,
            version: version.to_string(),
        };

        diesel::insert_into(factories::table)
            .values(&new_factory)
            .returning(Factory::as_returning())
            .get_result(conn)
            .expect("Error saving new factory")
    }

    /// Read factory by ID
    #[allow(dead_code)]
    pub fn read_factory(conn: &mut PgConnection, id: i32) -> Option<Factory> {
        factories::table
            .find(id)
            .select(Factory::as_select())
            .first(conn)
            .ok()
    }

    /// Get all factories from the database
    ///
    /// # Arguments
    /// * `conn` - Database connection
    ///
    /// # Returns
    /// Vector of all factory records
    ///
    /// # Panics
    /// * If database query fails
    /// * If factory records cannot be loaded
    #[allow(dead_code)]
    pub fn read_all_factories(conn: &mut PgConnection) -> Vec<Factory> {
        factories::table
            .select(Factory::as_select())
            .load(conn)
            .expect("Error loading factories")
    }

    /// Update Factory
    #[allow(dead_code)]
    pub fn update_factory(
        conn: &mut PgConnection,
        id: i32,
        name: &str,
        fee: i32,
    ) -> Option<Factory> {
        diesel::update(factories::table.find(id))
            .set((
                factories::name.eq(name),
                factories::fee.eq(fee),
            ))
            .returning(Factory::as_returning())
            .get_result(conn)
            .ok()
    }

    /// Delete Factory
    #[allow(dead_code)]
    pub fn delete_factory(conn: &mut PgConnection, id: i32) -> bool {
        diesel::delete(factories::table.find(id))
            .execute(conn)
            .is_ok()
    }
    
    /// Get or create a factory
    ///
    /// # Arguments
    /// * `conn` - Database connection
    /// * `address` - Factory contract address
    /// * `name` - Factory name
    /// * `fee` - Trading fee in basis points
    /// * `version` - Protocol version
    ///
    /// # Returns
    /// The factory record, either existing or newly created
    ///
    /// # Errors
    /// * If database operations fail
    /// * If factory creation violates constraints
    /// * If factory lookup fails
    #[allow(dead_code)]
    pub fn read_or_create(
        conn: &mut PgConnection,
        address: &str,
        name: &str,
        fee: i32,
        version: &str,
    ) -> Result<Factory, diesel::result::Error> {
        factories::table
            .filter(factories::address.eq(address))
            .first(conn)
            .or_else(|_| {
                let new_factory = NewFactory {
                    address: address.to_string(),
                    name: name.to_string(),
                    fee,
                    version: version.to_string(),
                };
                diesel::insert_into(factories::table)
                    .values(&new_factory)
                    .returning(Factory::as_returning())
                    .get_result(conn)
            })
    }
}

