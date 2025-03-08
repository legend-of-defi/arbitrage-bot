# Models module

This module contains the database models and schema definitions used throughout the application.

## Design Principles

1. **Type Safety**
   - Models use strong typing to catch errors at compile time
   - Custom types used where appropriate (e.g., Address for Ethereum addresses)

2. **Database Mapping**
   - Models map directly to database tables
   - Use Diesel's query builder DSL for type-safe queries
   - Follow Diesel conventions for model naming and structure

3. **Validation**
   - Models enforce data validation rules
   - Constructor methods validate inputs
   - Prevent invalid states through type system

4. **Encapsulation**
   - Internal implementation details hidden
   - Public interfaces expose only necessary functionality
   - Changes to database schema contained within models

The models provide a clean interface between the database and application logic while ensuring data integrity and type safety.
