use std::sync::Arc;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use tokio::sync::Barrier;

use alloy_primitives::{Address, B256, U256};
use rand::Rng;

use mev_relay_bench::bid::btree::BTreeBidStore;
use mev_relay_bench::bid::sorted_vec::SortedVecBidStore;
use mev_relay_bench::bid::BidStore;
use mev_relay_bench::types::{BidHeader, Slot};

fn make_bid(slot: Slot, value: u64) -> BidHeader {
    BidHeader {
        slot,
        builder: Address::random(),
        value: U256::from(value),
        block_hash: B256::random(),
        received_at_ns: mev_relay_bench::types::now_ns(),
    }
}

/// Bench: insert N bids into a single slot sequentially.
fn bench_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("insert");

    for n in [10, 50, 100, 500] {
        let bids: Vec<BidHeader> = (0..n)
            .map(|_| make_bid(1, rand::rng().random_range(1_000_000..100_000_000)))
            .collect();

        group.bench_with_input(BenchmarkId::new("BTreeMap", n), &bids, |b, bids| {
            b.iter(|| {
                let store = BTreeBidStore::new();
                for bid in bids {
                    store.insert(black_box(bid.clone()));
                }
            });
        });

        group.bench_with_input(BenchmarkId::new("SortedVec", n), &bids, |b, bids| {
            b.iter(|| {
                let store = SortedVecBidStore::new();
                for bid in bids {
                    store.insert(black_box(bid.clone()));
                }
            });
        });
    }

    group.finish();
}

/// Bench: get_best after N bids inserted.
fn bench_get_best(c: &mut Criterion) {
    let mut group = c.benchmark_group("get_best");

    for n in [10, 50, 100, 500] {
        let bids: Vec<BidHeader> = (0..n)
            .map(|_| make_bid(1, rand::rng().random_range(1_000_000..100_000_000)))
            .collect();

        let btree = BTreeBidStore::new();
        let svec = SortedVecBidStore::new();
        for bid in &bids {
            btree.insert(bid.clone());
            svec.insert(bid.clone());
        }

        group.bench_with_input(BenchmarkId::new("BTreeMap", n), &n, |b, _| {
            b.iter(|| black_box(btree.get_best(1)));
        });

        group.bench_with_input(BenchmarkId::new("SortedVec", n), &n, |b, _| {
            b.iter(|| black_box(svec.get_best(1)));
        });
    }

    group.finish();
}

/// Bench: concurrent inserts from M builders into same slot.
/// This is the real-world scenario — builders racing to submit bids.
fn bench_concurrent_insert(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let mut group = c.benchmark_group("concurrent_insert");

    for num_builders in [10, 50, 100] {
        group.bench_with_input(
            BenchmarkId::new("BTreeMap", num_builders),
            &num_builders,
            |b, &num_builders| {
                b.to_async(&rt).iter(|| async {
                    let store = Arc::new(BTreeBidStore::new());
                    let barrier = Arc::new(Barrier::new(num_builders));

                    let mut handles = Vec::new();
                    for _ in 0..num_builders {
                        let store = Arc::clone(&store);
                        let barrier = Arc::clone(&barrier);
                        handles.push(tokio::spawn(async move {
                            barrier.wait().await;
                            let val = rand::rng().random_range(1_000_000..100_000_000u64);
                            store.insert(make_bid(1, val));
                        }));
                    }
                    for h in handles {
                        h.await.unwrap();
                    }
                    black_box(store.get_best(1));
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("SortedVec", num_builders),
            &num_builders,
            |b, &num_builders| {
                b.to_async(&rt).iter(|| async {
                    let store = Arc::new(SortedVecBidStore::new());
                    let barrier = Arc::new(Barrier::new(num_builders));

                    let mut handles = Vec::new();
                    for _ in 0..num_builders {
                        let store = Arc::clone(&store);
                        let barrier = Arc::clone(&barrier);
                        handles.push(tokio::spawn(async move {
                            barrier.wait().await;
                            let val = rand::rng().random_range(1_000_000..100_000_000u64);
                            store.insert(make_bid(1, val));
                        }));
                    }
                    for h in handles {
                        h.await.unwrap();
                    }
                    black_box(store.get_best(1));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_insert, bench_get_best, bench_concurrent_insert);
criterion_main!(benches);