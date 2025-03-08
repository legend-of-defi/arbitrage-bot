# Sync Module

This module contains worker threads that synchronize our database with both on-chain and off-chain data sources.

## Naming Convention

Each worker thread is named after the entity it syncs in plural form (e.g., `sync::pairs`, `sync::events`), making it intuitive to reference them in code.

## Worker Pattern

Every sync worker must implement the following pattern:

1. Run in an infinite loop that:
   - Checks for work to be done
   - Executes the work if found
   - Sleeps for a configured period if no work is available
   - Repeats

## Architecture Principles

### 1. Simplicity
- Each worker does one thing and does it well
- Workers save data to the database as quickly as possible
- Focus on atomic operations that can be easily retried

### 2. Resilience
- Workers fail fast and recover gracefully
- No hard dependencies between workers
- Database can be temporarily incomplete - other workers will fill gaps
- Each worker can run independently and concurrently

### 3. Consistency
- Database must never be left in an inconsistent state
- Temporary incompleteness is acceptable and expected
- All operations should be idempotent

### 4. Management
- Workers are managed by a central thread pool
- Each worker can be started/stopped independently
- Resource usage is controlled through configuration

## Example Workers

- `sync::events`: Syncs on-chain events
- `sync::factory_pairs`: Syncs pairs from factory contracts
- `sync::pair_tokens`: Syncs token information for pairs
- `sync::reserves`: Syncs pair reserves

This architecture ensures our system stays synchronized with external data sources while maintaining resilience and consistency.