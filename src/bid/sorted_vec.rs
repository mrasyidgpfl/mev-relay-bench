use dashmap::DashMap;

use crate::types::{BidHeader, RankedBid, Slot};

use super::{BestBid, BidStore};

/// Sorted-vector bid store.
///
/// Bids kept in a Vec sorted by value ascending (best = last).
/// Uses binary search to find insertion point, then shifts elements.
///
/// Tradeoffs vs BTreeMap:
/// - O(n) insert (shift) vs O(log n), but n is small (10-100 builders)
/// - O(1) get_best (last element): same as BTreeMap
/// - Better cache locality: contiguous memory, no tree node pointers
/// - Lower memory overhead: no per-node left/right/parent/color fields
///
/// Hypothesis: faster than BTreeMap for realistic builder counts (<100)
/// due to cache effects. The benchmark will tell us.
pub struct SortedVecBidStore {
    slots: DashMap<Slot, Vec<RankedBid>>,
}

impl SortedVecBidStore {
    pub fn new() -> Self {
        Self {
            slots: DashMap::new(),
        }
    }
}

impl BidStore for SortedVecBidStore {
    fn insert(&self, bid: BidHeader) {
        let slot = bid.slot;
        let ranked = RankedBid::new(bid);

        let mut entry = self.slots.entry(slot).or_default();

        // Binary search for insertion point by value.
        let idx = entry
            .binary_search_by(|probe| {
                probe
                    .header
                    .value
                    .cmp(&ranked.header.value)
                    .then(std::cmp::Ordering::Less)
            })
            .unwrap_or_else(|i| i);

        entry.insert(idx, ranked);
    }

    fn get_best(&self, slot: Slot) -> Option<BestBid> {
        self.slots
            .get(&slot)
            .and_then(|bids| bids.last().map(BestBid::from))
    }

    fn get_top_n(&self, slot: Slot, n: usize) -> Vec<BestBid> {
        self.slots
            .get(&slot)
            .map(|bids| {
                bids.iter()
                    .rev()
                    .take(n)
                    .map(BestBid::from)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn count(&self, slot: Slot) -> usize {
        self.slots.get(&slot).map(|v| v.len()).unwrap_or(0)
    }

    fn prune_before(&self, slot: Slot) -> usize {
        let old: Vec<Slot> = self
            .slots
            .iter()
            .filter(|e| *e.key() < slot)
            .map(|e| *e.key())
            .collect();
        let n = old.len();
        for k in old {
            self.slots.remove(&k);
        }
        n
    }

    fn name(&self) -> &'static str {
        "Sorted Vec + DashMap"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{Address, B256, U256};

    fn make_bid(slot: Slot, value: u64) -> BidHeader {
        BidHeader {
            slot,
            builder: Address::random(),
            value: U256::from(value),
            block_hash: B256::random(),
            received_at_ns: crate::types::now_ns(),
        }
    }

    #[test]
    fn best_is_highest_value() {
        let store = SortedVecBidStore::new();
        store.insert(make_bid(1, 100));
        store.insert(make_bid(1, 500));
        store.insert(make_bid(1, 200));

        let best = store.get_best(1).unwrap();
        assert_eq!(best.value, U256::from(500));
    }

    #[test]
    fn maintains_sort_order() {
        let store = SortedVecBidStore::new();
        store.insert(make_bid(1, 300));
        store.insert(make_bid(1, 100));
        store.insert(make_bid(1, 500));
        store.insert(make_bid(1, 200));

        let top = store.get_top_n(1, 4);
        let vals: Vec<u64> = top.iter().map(|b| b.value.to::<u64>()).collect();
        assert_eq!(vals, vec![500, 300, 200, 100]);
    }

    #[test]
    fn empty_slot_returns_none() {
        let store = SortedVecBidStore::new();
        assert!(store.get_best(99).is_none());
    }

    #[test]
    fn prune_removes_old_slots() {
        let store = SortedVecBidStore::new();
        store.insert(make_bid(1, 100));
        store.insert(make_bid(2, 200));
        store.insert(make_bid(5, 500));

        let pruned = store.prune_before(3);
        assert_eq!(pruned, 2);
        assert!(store.get_best(1).is_none());
        assert!(store.get_best(5).is_some());
    }

    #[test]
    fn count_per_slot() {
        let store = SortedVecBidStore::new();
        store.insert(make_bid(1, 100));
        store.insert(make_bid(1, 200));
        store.insert(make_bid(2, 300));

        assert_eq!(store.count(1), 2);
        assert_eq!(store.count(2), 1);
    }
}