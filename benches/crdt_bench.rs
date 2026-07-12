use criterion::{criterion_group, criterion_main, Criterion};
use aether::crdt::{GCounter, GSet, PNCounter, ORSet};

fn benchmark_gcounter_merge(c: &mut Criterion) {
    let mut a = GCounter::new("node_a");
    let mut b = GCounter::new("node_b");
    a.increment_by(100);
    b.increment_by(200);

    c.bench_function("gcounter_merge", |bench| {
        bench.iter(|| {
            let mut temp = a.clone();
            temp.merge(&b);
        })
    });
}

fn benchmark_gset_merge(c: &mut Criterion) {
    let mut a = GSet::new();
    let mut b = GSet::new();
    for i in 0..100 {
        a.add(i);
        b.add(i + 50);
    }

    c.bench_function("gset_merge_100_items", |bench| {
        bench.iter(|| {
            let mut temp = a.clone();
            temp.merge(&b);
        })
    });
}

fn benchmark_orset_merge(c: &mut Criterion) {
    let mut a = ORSet::new("node_a");
    let mut b = ORSet::new("node_b");
    for i in 0..100 {
        a.add(format!("item-{}", i));
        b.add(format!("item-{}", i + 50));
    }

    c.bench_function("orset_merge_100_items", |bench| {
        bench.iter(|| {
            let mut temp = a.clone();
            temp.merge(&b);
        })
    });
}

criterion_group!(benches, benchmark_gcounter_merge, benchmark_gset_merge, benchmark_orset_merge);
criterion_main!(benches);
