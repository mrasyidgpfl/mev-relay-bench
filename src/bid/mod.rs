pub mod btree;
pub mod sorted_vec;

use crate::types::{BidHeader, RankedBid, Slot};
use alloy_primitives::U256;

/// Response returned by `get_best`, just the fields a proposer needs.
#[derive(Debug, Clone)]
pub struct BestBid {
    pub slot: Slot,
    pub value: U256,
    pub block_hash: alloy_primitives::B256,
    pub builder: alloy_primitives::Address,
}

impl From<&RankedBid> for BestBid {
    fn from(r: &RankedBid) -> Self {
        Self {
            slot: r.header.slot,
            value: r.header.value,
            block_hash: r.header.block_hash,
            builder: r.header.builder,
        }
    }
}

/// Trait for bid storage backends.
///
/// A relay must:
/// 1. Insert bids fast (called on every builder submission)
/// 2. Return the best bid for a slot fast (proposer's critical path)
/// 3. Prune old slots periodically
pub trait BidStore: Send + Sync {
    fn insert(&self, bid: BidHeader);
    fn get_best(&self, slot: Slot) -> Option<BestBid>;
    fn get_top_n(&self, slot: Slot, n: usize) -> Vec<BestBid>;
    fn count(&self, slot: Slot) -> usize;
    fn prune_before(&self, slot: Slot) -> usize;
    fn name(&self) -> &'static str;
}