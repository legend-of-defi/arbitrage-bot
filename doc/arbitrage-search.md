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

A significant majority (96.4%) of tokens are part of only one pool, typically WETH.

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

