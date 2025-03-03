# Arbitrage

Arbitrage calculations are done here.

## Design goals
1. Very narrow API surface
2. Maximum abstraction of external dependencies (alloy, database, etc.)
3. Data is passed via simple Value Objects
4. Most structs are either fully immutable or internally immutable

## API

### World

Created once:
```rust
let world = World::new(pools: &HashSet<Pool>);
```

This is a singleton struct that is created during startup. It is given
* `market: HashSet<Pool>` - pools with their reserves. The structure closely follow the structure of `Sync` event.

Updated every block:
```rust
let world_update: WorldUpdate = market.update(pools: &HashSet<Pool>);
```

The signature of this function is identical to the constructor. It is called once per block as long as that block has `Sync` events.

### WorldUpdate

`WorldUpdate` is an immutable instance that knows everything about arbitrage opportunities in the current block.
It has a bunch of methods that are mostly useful for logging and debugging. Practically speaking we will be using

```rust
    fn profitable_cycle_quotes(&self) -> Vec<CycleQuote> {
```
that resurns all profitable `CycleQuote`.

### CycleQuote

`CycleQuote` is a `Cycle` that has all the important numbers precalculated and has everything that is needed to execute the arbitrage.

It has these fuctions:
* `profit() I256` - not that it is signed generally speaking. You won't get quotes with negative profit here. This is
   for future cases where we may still want to do swaps even if unprofitable. For example, to rebalance our portfolio.
* `profit_margin() i32` - this is in basis points (hundredths of a percent), so 1234 = 12.34%. We just don't want to deal
   for floating point
* `is_profitable() bool` - again, this is for a future use case. Here it is guaranteed to be `true`
* `amount_in() I256`, `amount_out() I256` - should be self-explanatory
* `swap_quotes() &Vec<SwapQuote>` - vector of individual `SwapQuote`s that this `CycleQuote` is comprised of

### SwapQuote

`SwapQuote` is similarly a `Swap` with all the imporant number precalculated. Specifically
* `amount_in() U256`
* `amount_out() U256`
* `rate() f64`