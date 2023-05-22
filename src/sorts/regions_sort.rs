//! `regions_sort`
//!
//! Based on:
//! Omar Obeya, Endrias Kahssay, Edward Fan, and Julian Shun.
//! Theoretically-Efficient and Practical Parallel In-Place Radix Sorting.
//! In ACM Symposium on Parallelism in Algorithms and Architectures (SPAA), 2019.
//!
//! Summary:
//! 1. Split into buckets
//! 2. Compute counts for each bucket and sort each bucket in-place
//! 3. Generate global counts
//! 4. Generate Graph & Sort
//!     4.1 List outbound regions for each country
//!     4.2 For each country (C):
//!         4.2.1: List the inbounds for C (filter outbounds for each other country by destination: C)
//!         4.2.2: For each thread:
//!             4.2.2.1: Pop an item off the inbound (country: I) & outbound (country: O) queues for C
//!             4.2.2.2/a: If they are the same size, continue
//!             4.2.2.2/b: If I is bigger than O, keep the remainder of I in the queue and continue
//!             4.2.2.2/c: If O is bigger than I, keep the remainder of O in the queue and continue
//!             4.2.2.3: Swap items in C heading to O, with items in I destined for C (items in C may or may not be destined for O ultimately)
//!
//! ## Characteristics
//!
//!  * mostly in-place
//!  * multi-threaded
//!  * unstable
//!
//! ## Performance
//!
//! This typically performs worse than the other, simpler, multi-threaded algorithms such as
//! `recombinating_sort` and `scanning_sort`, however it uses a very clever and efficient algorithm
//! from a research paper that means for certain inputs and certain memory conditions it can provide
//! the best performance due to minimizing work spent copying and moving things.
//!
//! ## Notes
//!
//! This may not be entirely the same as the algorithm described by the research paper. Some things
//! did not seem to matter, and have been omitted for performance reasons.

use crate::sorter::Sorter;
use crate::sorts::ska_sort::ska_sort;
use crate::utils::*;
use crate::RadixKey;
use partition::partition_index;
use rayon::current_num_threads;
use rayon::prelude::*;
use std::cmp::{min, Ordering};

/// Operation represents a pair of edges, which have content slices that need to be swapped.
struct Operation<'bucket, T>(Edge<'bucket, T>, Edge<'bucket, T>);

/// Edge represents an outbound bit of data from a "country", an edge in the regions
/// graph. A "country" refers to a space which has been determined to be reserved for a particular
/// byte value in the final array.
struct Edge<'bucket, T> {
    /// dst is the destination country index
    dst: usize,
    /// init is the initial country index
    init: usize,
    slice: &'bucket mut [T],
}

/// generate_outbounds generates a Vec for each country containing all the outbound edges
/// for that country.
fn generate_outbounds<'bucket, T>(
    bucket: &'bucket mut [T],
    local_counts: &[[usize; 256]],
    global_counts: &[usize; 256],
) -> Vec<Edge<'bucket, T>> {
    let mut outbounds: Vec<Edge<T>> = Vec::new();
    let mut rem_bucket = bucket;
    let mut local_bucket = 0;
    let mut local_country = 0;
    let mut global_country = 0;
    let mut target_global_dist = global_counts[0];
    let mut target_local_dist = local_counts[0][0];

    while !(global_country == 255 && local_country == 255 && local_bucket == local_counts.len() - 1)
    {
        let step = min(target_global_dist, target_local_dist);

        // 1. Add the current step to the outbounds
        if step != 0 {
            let (slice, rem) = rem_bucket.split_at_mut(step);
            rem_bucket = rem;

            if local_country != global_country {
                outbounds.push(Edge {
                    dst: local_country,
                    init: global_country,
                    slice,
                });
            }
        }

        // 2. Update target_global_dist
        if step == target_global_dist && global_country < 255 {
            global_country += 1;
            target_global_dist = global_counts[global_country];
        } else {
            target_global_dist -= step;
        }

        // 3. Update target_local_dist
        if step == target_local_dist
            && !(local_bucket == local_counts.len() - 1 && local_country == 255)
        {
            if local_country < 255 {
                local_country += 1;
            } else {
                local_bucket += 1;
                local_country = 0;
            }

            target_local_dist = local_counts[local_bucket][local_country];
        } else {
            target_local_dist -= step;
        }
    }

    outbounds
}

/// list_operations takes the lists of outbounds and turns it into a list of swaps to perform
fn list_operations<T>(
    country: usize,
    mut outbounds: Vec<Edge<T>>,
) -> (Vec<Edge<T>>, Vec<Operation<T>>) {
    // 1. Extract current country outbounds from full outbounds list
    // NOTE(nathan): Partitioning a single array benched faster than
    // keeping an array per country (256 arrays total).
    let ob = partition_index(&mut outbounds, |e| e.init != country);
    let mut current_outbounds = outbounds.split_off(ob);

    // 2. Calculate inbounds for country
    let p = partition_index(&mut outbounds, |e| e.dst != country);
    let mut inbounds = outbounds.split_off(p);

    // 3. Pair up inbounds & outbounds into an operation, returning unmatched data to the working arrays
    let mut operations = Vec::new();

    loop {
        let i = match inbounds.pop() {
            Some(i) => i,
            None => {
                outbounds.append(&mut current_outbounds);
                break;
            }
        };

        let o = match current_outbounds.pop() {
            Some(o) => o,
            None => {
                outbounds.push(i);
                outbounds.append(&mut inbounds);
                break;
            }
        };

        let op = match i.slice.len().cmp(&o.slice.len()) {
            Ordering::Equal => Operation(i, o),
            Ordering::Less => {
                let (sl, rem) = o.slice.split_at_mut(i.slice.len());

                current_outbounds.push(Edge {
                    dst: o.dst,
                    init: o.init,
                    slice: rem,
                });

                let o = Edge {
                    dst: o.dst,
                    init: o.init,
                    slice: sl,
                };

                Operation(i, o)
            }
            Ordering::Greater => {
                let (sl, rem) = i.slice.split_at_mut(o.slice.len());

                inbounds.push(Edge {
                    dst: i.dst,
                    init: i.init,
                    slice: rem,
                });

                let i = Edge {
                    dst: i.dst,
                    init: i.init,
                    slice: sl,
                };

                Operation(i, o)
            }
        };

        operations.push(op);
    }

    // 4. Return the paired operations
    (outbounds, operations)
}

pub fn regions_sort<T>(
    bucket: &mut [T],
    counts: &[usize; 256],
    tile_counts: &[[usize; 256]],
    tile_size: usize,
    level: usize,
) where
    T: RadixKey + Sized + Send + Copy + Sync,
{
    let threads = current_num_threads();
    bucket
        .par_chunks_mut(tile_size)
        .zip(tile_counts.par_iter())
        .for_each(|(chunk, counts)| {
            let mut prefix_sums = get_prefix_sums(counts);
            let end_offsets = get_end_offsets(counts, &prefix_sums);
            ska_sort(chunk, &mut prefix_sums, &end_offsets, level);
        });

    let mut outbounds = generate_outbounds(bucket, tile_counts, counts);
    let mut operations = Vec::new();

    // This loop calculates and executes all operations that can be done in parallel, each pass.
    loop {
        if outbounds.is_empty() {
            break;
        }

        // List out all the operations that need to be executed in this pass
        for country in 0..256 {
            let (new_outbounds, mut new_ops) = list_operations(country, outbounds);
            outbounds = new_outbounds;
            operations.append(&mut new_ops);
        }

        if operations.is_empty() {
            break;
        }

        // Execute all operations, swapping the paired slices (inbound/outbound edges)
        let chunk_size = (operations.len() / threads) + 1;
        operations.par_chunks_mut(chunk_size).for_each(|chunk| {
            for Operation(o, i) in chunk {
                i.slice.swap_with_slice(o.slice)
            }
        });

        // Create new edges for edges that were swapped somewhere other than their final destination
        for Operation(i, mut o) in std::mem::take(&mut operations) {
            if o.dst != i.init {
                o.init = i.init;
                o.slice = i.slice;
                outbounds.push(o);
            }
        }
    }
}

impl<'a> Sorter<'a> {
    pub(crate) fn regions_sort_adapter<T>(
        &self,
        bucket: &mut [T],
        counts: &[usize; 256],
        tile_counts: &[[usize; 256]],
        tile_size: usize,
        level: usize,
    ) where
        T: RadixKey + Sized + Send + Copy + Sync,
    {
        if bucket.len() < 2 {
            return;
        }

        regions_sort(bucket, counts, tile_counts, tile_size, level);

        if level == 0 {
            return;
        }

        self.director(bucket, counts, level - 1);
    }
}

#[cfg(test)]
mod tests {
    use crate::sorter::Sorter;
    use crate::tuner::Algorithm;
    use crate::tuners::StandardTuner;
    use crate::utils::test_utils::{
        sort_comparison_suite, sort_single_algorithm, validate_u32_patterns, NumericTest,
    };
    use crate::utils::{aggregate_tile_counts, cdiv, get_tile_counts};
    use crate::RadixKey;
    use rayon::current_num_threads;

    fn test_regions_sort<T>(shift: T)
    where
        T: NumericTest<T>,
    {
        let sorter = Sorter::new(true, &StandardTuner);

        sort_comparison_suite(shift, |inputs| {
            if inputs.len() == 0 {
                return;
            }

            let tile_size = cdiv(inputs.len(), current_num_threads());
            let (tile_counts, _) = get_tile_counts(inputs, tile_size, T::LEVELS - 1);
            let counts = aggregate_tile_counts(&tile_counts);

            sorter.regions_sort_adapter(inputs, &counts, &tile_counts, tile_size, T::LEVELS - 1);
        });
    }

    #[test]
    pub fn test_u8() {
        test_regions_sort(0u8);
    }

    #[test]
    pub fn test_u16() {
        test_regions_sort(8u16);
    }

    #[test]
    pub fn test_u32() {
        test_regions_sort(16u32);
    }

    #[test]
    pub fn test_u64() {
        test_regions_sort(32u64);
    }

    #[test]
    pub fn test_u128() {
        test_regions_sort(64u128);
    }

    #[test]
    pub fn test_usize() {
        test_regions_sort(32usize);
    }

    #[test]
    pub fn test_basic_integration() {
        sort_single_algorithm::<u32>(1_000_000, Algorithm::Regions);
    }

    #[test]
    pub fn test_u32_patterns() {
        let sorter = Sorter::new(true, &StandardTuner);

        validate_u32_patterns(|inputs| {
            if inputs.len() == 0 {
                return;
            }

            let tile_size = cdiv(inputs.len(), current_num_threads());
            let (tile_counts, _) = get_tile_counts(inputs, tile_size, u32::LEVELS - 1);
            let counts = aggregate_tile_counts(&tile_counts);

            sorter.regions_sort_adapter(inputs, &counts, &tile_counts, tile_size, u32::LEVELS - 1);
        });
    }
}
