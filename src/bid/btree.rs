use std::collections::BTreeMap;

use dashmap::DashMap;

use crate::types::{BidHeader, RankedBid, Slot};

use super::{BestBid, BidStore};

/// BTreeMap-backed bid store.
///
/// Outer: DashMap<Slot, BTreeMap<BidKey, RankedBid>>
/// - DashMap shards by slot, so different slots don't contend
/// - BTreeMap keeps bids sorted by value within a slot
/// - last() gives us the best bid in O(1)
pub struct BTreeBidStore {
    slots: DashMap<Slot, BTreeMap<BidKey, RankedBid>>,
}

/// Composite key so BTreeMap sorts by value, with block_hash
/// as tiebreaker when two builders bid the same amount.
#[derive(Debug, Clone, Eq, PartialEq)]
struct BidKey {
    value_bytes: [u8; 32],
    block_hash_bytes: [u8; 32],
}

impl Ord for BidKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.value_bytes
            .cmp(&other.value_bytes)
            .then(self.block_hash_bytes.cmp(&other.block_hash_bytes))
    }
}

impl PartialOrd for BidKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl BidKey {
    fn from_header(h: &BidHeader) -> Self {
        Self {
            value_bytes: h.value.to_be_bytes(),
            block_hash_bytes: h.block_hash.0,
        }
    }
}

impl BTreeBidStore {
    pub fn new() -> Self {
        Self {
            slots: DashMap::new(),
        }
    }
}

impl BidStore for BTreeBidStore {
    fn insert(&self, bid: BidHeader) {
        let slot = bid.slot;
        let key = BidKey::from_header(&bid);
        let ranked = RankedBid::new(bid);
        self.slots.entry(slot).or_default().insert(key, ranked);
    }

    fn get_best(&self, slot: Slot) -> Option<BestBid> {
        self.slots
            .get(&slot)
            .and_then(|tree| tree.last_key_value().map(|(_, bid)| BestBid::from(bid)))
    }

    fn get_top_n(&self, slot: Slot, n: usize) -> Vec<BestBid> {
        self.slots
            .get(&slot)
            .map(|tree| {
                tree.iter()
                    .rev()
                    .take(n)
                    .map(|(_, bid)| BestBid::from(bid))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn count(&self, slot: Slot) -> usize {
        self.slots.get(&slot).map(|tree| tree.len()).unwrap_or(0)
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
        "BTreeMap + DashMap"
    }
}