use criterion::*;
use nanorand::{Rng, WyRand};
use rdst::utils::*;
use rdst::sorts::ska_sort::ska_sort;
use rdst::sorts::scanning_radix_sort::scanning_radix_sort;
use rdst::sorts::lsb_radix_sort::lsb_radix_sort_adapter;
use rdst::tuning_parameters::TuningParameters;
use std::time::Duration;

fn counts(c: &mut Criterion) {
    let n = 500_000_000;
    let mut inputs = Vec::with_capacity(n);
    let mut rng = WyRand::new();

    for _ in 0..n {
        inputs.push(rng.generate::<u32>());
    }

    let input_sets: Vec<Vec<u32>> = vec![
        inputs.clone(),
        inputs[..200_000_000].to_vec(),
        inputs[..100_000_000].to_vec(),
        inputs[..50_000_000].to_vec(),
        inputs[..10_000_000].to_vec(),
        inputs[..5_000_000].to_vec(),
        inputs[..2_000_000].to_vec(),
        inputs[..1_000_000].to_vec(),
        inputs[..500_000].to_vec(),
        inputs[..450_000].to_vec(),
        inputs[..400_000].to_vec(),
        inputs[..350_000].to_vec(),
        inputs[..300_000].to_vec(),
        inputs[..200_000].to_vec(),
        inputs[..100_000].to_vec(),
        inputs[..50_000].to_vec(),
        inputs[..10_000].to_vec(),
        inputs[..5_000].to_vec(),
    ];

    drop(inputs);

    let mut group = c.benchmark_group("counts");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(5));
    group.warm_up_time(Duration::from_secs(1));
    for set in input_sets.iter() {
        let l = set.len();
        group.throughput(Throughput::Elements(l as u64));
        group.bench_with_input(BenchmarkId::new("get_counts", l), set, |bench, set| {
            bench.iter_batched(
                || set.clone(),
                |input| {
                    let c = get_counts(&input, 0);
                    black_box(c);
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_with_input(BenchmarkId::new("par_get_counts", l), set, |bench, set| {
            bench.iter_batched(
                || set.clone(),
                |input| {
                    let c = par_get_counts(&input, 0);
                    black_box(c);
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn scanning_sort(c: &mut Criterion) {
    let n = 200_000_000;
    let mut inputs = Vec::with_capacity(n);
    let mut rng = WyRand::new();
    let tuning = TuningParameters::new(4);

    for _ in 0..n {
        inputs.push(rng.generate::<u32>());
    }

    let input_sets: Vec<Vec<u32>> = vec![
        inputs.clone(),
        inputs[..100_000_000].to_vec(),
        inputs[..50_000_000].to_vec(),
        inputs[..10_000_000].to_vec(),
        inputs[..5_000_000].to_vec(),
        inputs[..2_000_000].to_vec(),
        inputs[..1_000_000].to_vec(),
        inputs[..500_000].to_vec(),
        inputs[..300_000].to_vec(),
        inputs[..200_000].to_vec(),
        inputs[..100_000].to_vec(),
        inputs[..50_000].to_vec(),
        inputs[..10_000].to_vec(),
        inputs[..5_000].to_vec(),
    ];

    drop(inputs);

    let mut group = c.benchmark_group("scanning_sort_level_4");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(5));
    group.warm_up_time(Duration::from_secs(1));
    for set in input_sets.iter() {
        let l = set.len();
        group.throughput(Throughput::Elements(l as u64));
        group.bench_with_input(
            BenchmarkId::new("scanning_radix_sort", l),
            set,
            |bench, set| {
                bench.iter_batched(
                    || set.clone(),
                    |mut input| {
                        scanning_radix_sort(&tuning, &mut input, 3, false);
                        black_box(input);
                    },
                    BatchSize::SmallInput,
                );
            },
        );

        group.bench_with_input(BenchmarkId::new("lsb_radix_sort", l), set, |bench, set| {
            bench.iter_batched(
                || set.clone(),
                |mut input| {
                    lsb_radix_sort_adapter(&mut input, 0, 3);
                    black_box(input);
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_ska_sort(c: &mut Criterion) {
    let n = 10_000_000;
    let mut inputs = Vec::with_capacity(n);
    let mut rng = WyRand::new();
    let tuning = TuningParameters::new(8);

    for _ in 0..n {
        inputs.push(rng.generate::<u32>());
    }

    let input_sets: Vec<Vec<u32>> = vec![
        inputs.clone(),
        inputs[..5_000_000].to_vec(),
        inputs[..2_000_000].to_vec(),
        inputs[..1_000_000].to_vec(),
        inputs[..500_000].to_vec(),
        inputs[..300_000].to_vec(),
        inputs[..200_000].to_vec(),
        inputs[..100_000].to_vec(),
        inputs[..50_000].to_vec(),
        inputs[..10_000].to_vec(),
        inputs[..5_000].to_vec(),
    ];

    drop(inputs);

    let mut group = c.benchmark_group("ska_sort_level_4");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(5));
    group.warm_up_time(Duration::from_secs(1));
    for set in input_sets.iter() {
        let l = set.len();
        group.throughput(Throughput::Elements(l as u64));
        group.bench_with_input(BenchmarkId::new("ska_sort", l), set, |bench, set| {
            bench.iter_batched(
                || set.clone(),
                |mut input| {
                    //ska_sort(&tuning, &mut input, 3, false);
                    black_box(input);
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_with_input(BenchmarkId::new("lsb_radix_sort", l), set, |bench, set| {
            bench.iter_batched(
                || set.clone(),
                |mut input| {
                    lsb_radix_sort_adapter(&mut input, 0, 3);
                    black_box(input);
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();

    let mut inputs = Vec::with_capacity(n);

    for _ in 0..n {
        inputs.push(rng.generate::<u64>());
    }

    let input_sets: Vec<Vec<u64>> = vec![
        inputs.clone(),
        inputs[..5_000_000].to_vec(),
        inputs[..2_000_000].to_vec(),
        inputs[..1_000_000].to_vec(),
        inputs[..500_000].to_vec(),
        inputs[..300_000].to_vec(),
        inputs[..200_000].to_vec(),
        inputs[..100_000].to_vec(),
        inputs[..50_000].to_vec(),
        inputs[..10_000].to_vec(),
        inputs[..5_000].to_vec(),
    ];

    drop(inputs);

    let mut group = c.benchmark_group("ska_sort_level_8");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(5));
    group.warm_up_time(Duration::from_secs(1));
    for set in input_sets.iter() {
        let l = set.len();
        group.throughput(Throughput::Elements(l as u64));
        group.bench_with_input(BenchmarkId::new("ska_sort", l), set, |bench, set| {
            bench.iter_batched(
                || set.clone(),
                |mut input| {
                    //ska_sort(&tuning, &mut input, 3, false);
                    black_box(input);
                },
                BatchSize::SmallInput,
            );
        });

        group.bench_with_input(BenchmarkId::new("lsb_radix_sort", l), set, |bench, set| {
            bench.iter_batched(
                || set.clone(),
                |mut input| {
                    lsb_radix_sort_adapter(&mut input, 0, 3);
                    black_box(input);
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

criterion_group!(tuning_parameters, counts, scanning_sort, bench_ska_sort);
criterion_main!(tuning_parameters);
