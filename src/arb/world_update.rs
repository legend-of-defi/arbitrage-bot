/// WIP The cycles that were affected by the update
pub struct WorldUpdate {
    // The cycles that were affected by the update
    // cycles: Vec<Cycle>,
}

// impl WorldUpdate {
//     /// Creates a new `WorldUpdate` with the given cycles.
//     pub const fn new(cycles: Vec<Cycle>) -> Self {
//         Self { cycles }
//     }

//     /// Returns a reference to the cycles in this update.
//     pub const fn cycles(&self) -> &Vec<Cycle> {
//         &self.cycles
//     }

//     /// Checks if all cycles in this update have reserves.
//     pub fn has_all_reserves(&self) -> bool {
//         self.cycles
//             .iter()
//             .all(super::cycle::Cycle::has_all_reserves)
//     }

//     /// Returns a list of swaps in the cycles that have no reserves.
//     pub fn swaps_with_no_reserves(&self) -> Vec<Swap> {
//         self.cycles
//             .iter()
//             .flat_map(super::cycle::Cycle::swaps_with_no_reserves)
//             .collect()
//     }

//     /// Best cycle quotes - the best quote for each cycle. Not necessarily exploitable.
//     pub fn best_cycle_quotes(&self) -> Vec<Result<CycleQuote, Error>> {
//         assert!(self.has_all_reserves(), "All cycles must have reserves");

//         self.cycles
//             .iter()
//             .map(super::cycle::Cycle::best_quote)
//             .collect()
//     }

//     /// Positive rate cycles - the cycles that have a positive rate.
//     fn positive_cycles(&self) -> Vec<Cycle> {
//         assert!(self.has_all_reserves(), "All cycles must have reserves");

//         self.cycles
//             .iter()
//             .filter(|cycle| cycle.is_positive())
//             .cloned()
//             .collect()
//     }

//     /// Profitable cycles - the cycles that have a positive rate and are exploitable
//     fn profitable_cycles(&self) -> Vec<Cycle> {
//         assert!(self.has_all_reserves(), "All cycles must have reserves");

//         // SAFETY: we know the cycles have reserves because we checked it
//         #[allow(clippy::unwrap_used)]
//         self.positive_cycles()
//             .iter()
//             .filter(|cycle| cycle.best_quote().is_ok() && cycle.best_quote().unwrap().is_profitable())
//             .cloned()
//             .collect()
//     }

//     /// Unprofitable cycles - the cycles that have a negative rate or are not exploitable
//     pub fn unprofitable_cycles(&self) -> Vec<Cycle> {
//         assert!(self.has_all_reserves(), "All cycles must have reserves");

//         // SAFETY: we know the cycles have reserves because we checked it
//         #[allow(clippy::unwrap_used)]
//         self.cycles()
//             .iter()
//             .filter(|cycle| {
//                 cycle.best_quote().is_ok() && !cycle.best_quote().unwrap().is_profitable()
//             })
//             .cloned()
//             .collect()
//     }

//     /// Profitable cycle quotes - the best quote for each profitable cycle.
//     pub fn profitable_cycle_quotes(&self) -> Vec<Result<CycleQuote, Error>> {
//         self.profitable_cycles()
//             .iter()
//             .map(super::cycle::Cycle::best_quote)
//             .collect()
//     }
// }

// #[cfg(test)]
// #[allow(clippy::unwrap_used)]
// mod tests {
//     use alloy::primitives::{I256, U256};

//     use crate::arb::test_helpers::{bare_swap, cycle, swap};

//     use super::*;

//     #[test]
//     fn test_has_all_reserves_is_true() {
//         let world_update = WorldUpdate::new(vec![
//             cycle(&[("F1", "A", "B", 100, 200), ("F2", "B", "A", 200, 100)]).unwrap(),
//             cycle(&[("F2", "A", "B", 100, 200), ("F1", "B", "A", 200, 100)]).unwrap(),
//         ]);
//         assert!(world_update.has_all_reserves());
//     }

//     #[test]
//     fn test_has_all_reserves_is_false() {
//         let world_update = WorldUpdate::new(vec![Cycle::new(Vec::from([
//             bare_swap("F1", "A", "B"),
//             bare_swap("F2", "B", "A"),
//         ]))
//         .unwrap()]);
//         assert!(!world_update.has_all_reserves());
//     }

//     #[test]
//     fn test_swaps_with_no_reserves() {
//         let world_update = WorldUpdate::new(vec![Cycle::new(Vec::from([
//             bare_swap("F1", "A", "B"),
//             bare_swap("F2", "B", "A"),
//         ]))
//         .unwrap()]);
//         assert_eq!(
//             world_update.swaps_with_no_reserves(),
//             vec![bare_swap("F1", "A", "B"), bare_swap("F2", "B", "A")]
//         );

//         let world_update = WorldUpdate::new(vec![Cycle::new(Vec::from([
//             swap("F1", "A", "B", 100, 200),
//             swap("F2", "B", "A", 200, 100),
//         ]))
//         .unwrap()]);
//         assert_eq!(world_update.swaps_with_no_reserves(), vec![]);
//     }

//     #[test]
//     fn test_profitable_cycles() {
//         // Unprofitable update
//         let world_update = WorldUpdate::new(vec![cycle(&[
//             ("F1", "A", "B", 100_000_000, 200_000_000),
//             ("F2", "B", "A", 200_000_000, 100_000_000),
//         ])
//         .unwrap()]);
//         assert!(world_update.profitable_cycles().is_empty());

//         // Slightly profitable update
//         let world_update = WorldUpdate::new(vec![
//             cycle(&[
//                 ("F1", "A", "B", 100_000_000, 200_000_000),
//                 ("F2", "B", "A", 200_000_000, 101_000_000),
//             ])
//             .unwrap(),
//             cycle(&[
//                 ("F1", "B", "A", 200_000_000, 100_000_000),
//                 ("F2", "A", "B", 101_000_000, 200_000_000),
//             ])
//             .unwrap(),
//         ]);
//         assert_eq!(world_update.cycles().len(), 2);

//         // Profitable cycle
//         assert_eq!(world_update.profitable_cycles().len(), 1);
//         let profitable_cycles = world_update.profitable_cycles();
//         let cycle = profitable_cycles.first().unwrap();
//         assert!(cycle.is_positive());

//         let best_quote = cycle.best_quote().unwrap();
//         assert!(best_quote.is_profitable());
//         assert_eq!(best_quote.amount_in(), U256::from(13354));
//         assert_eq!(best_quote.amount_out(), U256::from(13403));
//         assert_eq!(best_quote.profit(), I256::from_raw(U256::from(49)));

//         // Unprofitable cycle
//         assert_eq!(world_update.unprofitable_cycles().len(), 1);
//         let unprofitable_cycles = world_update.unprofitable_cycles();
//         let cycle = unprofitable_cycles.first().unwrap();
//         assert!(!cycle.is_positive());

//         let best_quote = cycle.best_quote().unwrap();
//         assert!(!best_quote.is_profitable());
//         assert_eq!(best_quote.amount_in(), U256::from(0));
//         assert_eq!(best_quote.amount_out(), U256::from(0));
//         assert_eq!(best_quote.profit(), I256::from_raw(U256::from(0)));
//     }
// }
