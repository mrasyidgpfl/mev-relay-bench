use alloy_primitives::{Address, B256, U256};
use serde::{Deserialize, Serialize};

/// Beacon chain slot number. One slot = 12 seconds on mainnet.
pub type Slot = u64;

/// A bid header submitted by a builder to the relay.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BidHeader {
    pub slot: Slot,
    pub builder: Address,
    pub value: U256,
    pub block_hash: B256,
    pub received_at_ns: u64,
}

/// A validated bid stored in the relay's ranking structure.
#[derive(Debug, Clone)]
pub struct RankedBid {
    pub header: BidHeader,
    pub indexed_at_ns: u64,
}

impl RankedBid {
    pub fn new(header: BidHeader) -> Self {
        Self {
            indexed_at_ns: now_ns(),
            header,
        }
    }
}

/// Nanosecond timestamp from system clock.
pub fn now_ns() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64
}