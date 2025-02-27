* This is a high-level research document outlining strategies for optimizing arbitrage detection.

# Analysis

Analysis based on 399,781 Uniswap V2 pools on Ethereum.

## Observations

### Sparse Graph Structure
There are 391,414 unique tokens, making this graph highly sparse—99.995% of token pairs/nodes lack pools/edges.

### Central Node
WETH serves as the central node, directly connecting to 98.6% of other tokens. The next most connected tokens are:
- USDC - 0.88%
- USDT - 0.75%
- DAI  - 0.26%

A significant majority (96.4%) of tokens are part of only one pool - WETH.

### Static Nature
New pools/nodes are rarely added. However, pool rates (node weights) fluctuate at a rate of 10-40 updates per block.

# Implementation Strategy

Optimizing for:
- **Compute efficiency**: Managing CPU, RAM, and disk usage.
- **Performance**: Calculations must be completed within a single block time (2 seconds on Base), preferably much faster.

Detecting 2-leg cycles is relatively straightforward and computationally manageable. However, detecting 3-leg cycles introduces an exponential increase in complexity due to combinatorial explosion, making 4+ cycles even more resource-intensive.

## Pruning Strategy

Aggressively remove inactive pools. Many pools remain idle for extended periods (e.g., https://etherscan.io/address/0f8e31593857e182fab1b7bf38ae269ece69f4e1, last swap 1220 days ago, GRID-WETH).

The criteria for pruning—minimum reserve thresholds and maximum inactivity periods—will balance node reduction against computational cost.

### Inactivity Period

This is the time between the last swap and the current block. This is harder to get due to a required chain scan.
Another option would be to use one of many services that provide this data. However, I think there is a better way: pool reserves.

### Pool Reserves

The pool reserves can be efficiently retrieved from the chain and they are a good proxy for the pool's activity.
The reason is simple: inactive pools won't have reserves because liquidity providers won't provide liquidity to them.

The question is: what is the minimum reserve threshold? Here is an analysis:

When we swap we want our price impact to be less than our profit margin. That is, if we identified a cycle that gives us 1% profit, we don't want to take a 2% hit on our swap. This means that the size of our swap depends on the
available liquidity in the pool.

The table below shows the dependency between the swap size as a percentage of pool reserves and the slippage it causes.

Swap size (% of reserves) | Slippage %
-----------------------------------------
                0.10% |       0.80%
                0.50% |       1.60%
                1.00% |       2.55%
                2.00% |       4.41%
                3.00% |       6.20%
                4.00% |       7.94%
                5.00% |       9.60%

The big question here is: what is the realistic profit margin we can expect from a typical arbitrage opportunity?
If it is 5% it means we can't swap more that 2% of the pool reserves without causing slippage to exceed our profit margin.

Then, we also need to account for pool fees - 0.3% per swap. For triangular arbitrage, we have 3 swaps per cycle, so 0.9% fee. I assume from now on that can afford ~2.5% slippage at most which means we can only swap 1% of the pool reserves.

Then comes gas fees. Our current `SimpleExecutor` implementation has worst case gas consumption of 385,366 per 2 swap cycle.
Base gas fees are between 0.1 and 0.5 gwei per gas unit. We'll use 0.25. This results in ~$0.25 per our contract execution at current Ethereum prices (~$2,700). I'll ignore the fact that the contract can be optimized a lot, I'll also ignore the fact that the gas price can also spike to ~$0.5. $0.25 is a good middle ground.

This means that if we swap ~1% and receive ~2.5% profit margin we need to get at least $0.25 profit to be even. Simple math tells us that we need to sweep $10. This means that the pool must have at least $1000 in reserves. At this is only to cover the gas fees.

This is a good proxy for the minimum pool size that we can consider.

## Cycle Precalculation

Precompute cycles and store them persistently. At startup:
- Read all pool reserves.
- Update cycle rates (product of swaps).
- Store values in memory for rapid updates per block.

The core data structure:
`HashMap<Swap, Vec<Cycle>>`, mapping each pool to its associated cycles. Only cycles containing updated pools need recalculation upon `Sync` events.

## Algorithm (Pseudo-Code):

### Periodic Tasks:
- Prune pools to remove inactive or low-liquidity pools.
- Precompute and persist 2-leg and 3-leg cycles (potentially more).

### Startup Initialization:
- Load all pools from the database.
- Retrieve swap rates from contracts and store `ln(reserve1/reserve0)`. Using logarithms converts multiplication into addition, optimizing calculations and transforming the arbitrage condition from `rate > 1` to `ln(rate) > 0`.
- For each pool, create two swaps: forward and reverse, with `-ln(rate)`.
- Load all cycles and update `Cycle.ln(rate) = cycle.swaps.sum(|p| p.ln(rate))`.
  - A cycle's rate depends solely on its constituent swaps and updates only when one of these swaps changes.
- Retain this data in memory for fast access.

### On Each `Sync` Event:
- Compute `Sync.ln(rate) = Sync.ln(reserve1/reserve0)`, updating both forward and reverse swap rates.
- Determine the difference: `Swap.ln(diff) = Sync.ln(rate) - Swap.ln(rate)`.
- Apply this difference to all cycles containing the affected swaps: `Cycle.ln(rate) += Swap.ln(diff)`.
- Track all impacted cycles.

### Per Block:
- Identify all affected cycles.
- Filter cycles where `Cycle.ln(rate) > 0`.
- These cycles represent potential arbitrage opportunities.

