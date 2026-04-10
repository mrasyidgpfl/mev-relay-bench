use std::sync::Arc;
use std::time::Instant;

use clap::Parser;
use tokio::sync::Barrier;

use alloy_primitives::{Address, B256, U256};
use rand::Rng;

use mev_relay_bench::bid::btree::BTreeBidStore;
use mev_relay_bench::bid::sorted_vec::SortedVecBidStore;
use mev_relay_bench::bid::BidStore;
use mev_relay_bench::types::{BidHeader, Slot};

#[derive(Parser)]
#[command(name = "mev-relay-bench")]
#[command(about = "Benchmark bid store backends for MEV-Boost relay performance")]
struct Cli {
    /// Number of simulated builders.
    #[arg(short, long, default_value = "20")]
    builders: usize,

    /// Number of slots to simulate.
    #[arg(short, long, default_value = "10")]
    slots: usize,

    /// Bids per builder per slot.
    #[arg(short = 'n', long, default_value = "3")]
    bids_per_slot: usize,
}

fn make_bid(slot: Slot, value: u64) -> BidHeader {
    BidHeader {
        slot,
        builder: Address::random(),
        value: U256::from(value),
        block_hash: B256::random(),
        received_at_ns: mev_relay_bench::types::now_ns(),
    }
}

/// Run a simulation: N builders concurrently insert bids per slot,
/// then measure get_best latency. Returns (insert_total_us, get_best_ns).
async fn run_sim<S: BidStore + 'static>(
    store: Arc<S>,
    slots: usize,
    builders: usize,
    bids_per: usize,
) -> (Vec<f64>, Vec<f64>) {
    let mut insert_times = Vec::new();
    let mut get_best_times = Vec::new();

    for slot in 1..=slots as u64 {
        let barrier = Arc::new(Barrier::new(builders));
        let mut handles = Vec::new();

        let insert_start = Instant::now();

        for _ in 0..builders {
            let store = Arc::clone(&store);
            let barrier = Arc::clone(&barrier);
            handles.push(tokio::spawn(async move {
                barrier.wait().await;
                for _ in 0..bids_per {
                    let val = rand::rng().random_range(1_000_000..100_000_000u64);
                    store.insert(make_bid(slot, val));
                }
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        let insert_us = insert_start.elapsed().as_nanos() as f64 / 1_000.0;
        insert_times.push(insert_us);

        let get_start = Instant::now();
        let _best = store.get_best(slot);
        let get_ns = get_start.elapsed().as_nanos() as f64;
        get_best_times.push(get_ns);
    }

    (insert_times, get_best_times)
}

fn stats(vals: &[f64]) -> (f64, f64, f64) {
    let mut sorted = vals.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let len = sorted.len();
    let avg = sorted.iter().sum::<f64>() / len as f64;
    let p50 = sorted[len / 2];
    let p99 = sorted[((len as f64) * 0.99) as usize].min(sorted[len - 1]);
    (avg, p50, p99)
}

fn print_row(name: &str, unit: &str, vals: &[f64]) {
    let (avg, p50, p99) = stats(vals);
    println!("  {name:<28} avg={avg:>10.1}{unit}  p50={p50:>10.1}{unit}  p99={p99:>10.1}{unit}");
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let total_bids = cli.builders * cli.bids_per_slot;

    println!();
    println!("mev-relay-bench");
    println!("===============");
    println!(
        "  {} builders × {} bids/builder × {} slots = {} bids/slot",
        cli.builders, cli.bids_per_slot, cli.slots, total_bids
    );
    println!();

    // BTreeMap
    let store = Arc::new(BTreeBidStore::new());
    let (ins, get) = run_sim(store, cli.slots, cli.builders, cli.bids_per_slot).await;
    println!("[BTreeMap + DashMap]");
    print_row("concurrent insert/slot", "μs", &ins);
    print_row("get_best", "ns", &get);
    println!();

    // SortedVec
    let store = Arc::new(SortedVecBidStore::new());
    let (ins, get) = run_sim(store, cli.slots, cli.builders, cli.bids_per_slot).await;
    println!("[Sorted Vec + DashMap]");
    print_row("concurrent insert/slot", "μs", &ins);
    print_row("get_best", "ns", &get);
    println!();
}