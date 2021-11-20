use crate::director::director;
use crate::tuner::Tuner;
use crate::utils::*;
use crate::RadixKey;
use arbitrary_chunks::ArbitraryChunks;
use rayon::prelude::*;

pub fn mt_lsb_sort<T>(
    src_bucket: &mut [T],
    dst_bucket: &mut [T],
    tile_counts: &[[usize; 256]],
    tile_size: usize,
    level: usize,
) where
    T: RadixKey + Sized + Send + Copy + Sync,
{
    let tiles = tile_counts.len();
    let mut minor_counts = Vec::with_capacity(256 * tiles);

    for b in 0..256 {
        for c in 0..tiles {
            minor_counts.push(tile_counts[c][b]);
        }
    }

    let mut chunks: Vec<&mut [T]> = dst_bucket.arbitrary_chunks_mut(minor_counts).collect();
    chunks.reverse();

    let mut collated_chunks: Vec<Vec<&mut [T]>> = Vec::with_capacity(tiles);
    collated_chunks.resize_with(tiles, || Vec::new());

    for _ in 0..256 {
        for t in 0..tiles {
            collated_chunks[t].push(chunks.pop().unwrap());
        }
    }

    collated_chunks
        .into_par_iter()
        .zip(src_bucket.par_chunks(tile_size))
        .for_each(|(mut buckets, bucket)| {
            if bucket.len() == 0 {
                return;
            }

            let mut offsets = [0usize; 256];
            let mut ends = [0usize; 256];

            for (i, b) in buckets.iter().enumerate() {
                if b.len() == 0 {
                    continue;
                }

                ends[i] = b.len() - 1;
            }

            let mut left = 0;
            let mut right = bucket.len() - 1;
            let pre = bucket.len() % 8;

            for _ in 0..pre {
                let b = bucket[right].get_level(level) as usize;

                buckets[b][ends[b]] = bucket[right];
                ends[b] = ends[b].saturating_sub(1);
                right = right.saturating_sub(1);
            }

            if pre == bucket.len() {
                return;
            }

            loop {
                if left >= right {
                    break;
                }

                let bl_0 = bucket[left].get_level(level) as usize;
                let bl_1 = bucket[left + 1].get_level(level) as usize;
                let bl_2 = bucket[left + 2].get_level(level) as usize;
                let bl_3 = bucket[left + 3].get_level(level) as usize;
                let br_0 = bucket[right].get_level(level) as usize;
                let br_1 = bucket[right - 1].get_level(level) as usize;
                let br_2 = bucket[right - 2].get_level(level) as usize;
                let br_3 = bucket[right - 3].get_level(level) as usize;

                buckets[bl_0][offsets[bl_0]] = bucket[left];
                offsets[bl_0] += 1;
                buckets[br_0][ends[br_0]] = bucket[right];
                ends[br_0] = ends[br_0].saturating_sub(1);
                buckets[bl_1][offsets[bl_1]] = bucket[left + 1];
                offsets[bl_1] += 1;
                buckets[br_1][ends[br_1]] = bucket[right - 1];
                ends[br_1] = ends[br_1].saturating_sub(1);
                buckets[bl_2][offsets[bl_2]] = bucket[left + 2];
                offsets[bl_2] += 1;
                buckets[br_2][ends[br_2]] = bucket[right - 2];
                ends[br_2] = ends[br_2].saturating_sub(1);
                buckets[bl_3][offsets[bl_3]] = bucket[left + 3];
                offsets[bl_3] += 1;
                buckets[br_3][ends[br_3]] = bucket[right - 3];
                ends[br_3] = ends[br_3].saturating_sub(1);

                left += 4;
                right = right.saturating_sub(4);
            }
        });
}

pub fn mt_lsb_sort_adapter<T>(
    bucket: &mut [T],
    start_level: usize,
    end_level: usize,
    tile_size: usize,
) where
    T: RadixKey + Sized + Send + Copy + Sync,
{
    if bucket.len() < 2 {
        return;
    }

    let mut tmp_bucket = get_tmp_bucket(bucket.len());
    let levels: Vec<usize> = (start_level..=end_level).into_iter().collect();
    let mut invert = false;

    for level in levels {
        let tile_counts = if invert {
            get_tile_counts(&tmp_bucket, tile_size, level)
        } else {
            get_tile_counts(bucket, tile_size, level)
        };

        if invert {
            mt_lsb_sort(&mut tmp_bucket, bucket, &tile_counts, tile_size, level)
        } else {
            mt_lsb_sort(bucket, &mut tmp_bucket, &tile_counts, tile_size, level)
        };

        invert = !invert;
    }

    if invert {
        bucket
            .par_chunks_mut(tile_size)
            .zip(tmp_bucket.par_chunks(tile_size))
            .for_each(|(chunk, tmp_chunk)| {
                chunk.copy_from_slice(tmp_chunk);
            });
    }
}

pub fn mt_oop_sort_adapter<T>(
    tuner: &(dyn Tuner + Send + Sync),
    in_place: bool,
    bucket: &mut [T],
    level: usize,
    counts: &[usize; 256],
    tile_counts: &[[usize; 256]],
    tile_size: usize,
) where
    T: RadixKey + Sized + Send + Copy + Sync,
{
    if bucket.len() <= 1 {
        return;
    }

    let mut tmp_bucket = get_tmp_bucket(bucket.len());
    mt_lsb_sort(bucket, &mut tmp_bucket, tile_counts, tile_size, level);

    bucket
        .par_chunks_mut(tile_size)
        .zip(tmp_bucket.par_chunks(tile_size))
        .for_each(|(chunk, tmp_chunk)| {
            chunk.copy_from_slice(tmp_chunk);
        });

    drop(tmp_bucket);

    director(tuner, in_place, bucket, counts.to_vec(), level - 1);
}

#[cfg(test)]
mod tests {
    use crate::sorts::mt_lsb_sort::mt_lsb_sort_adapter;
    use crate::test_utils::{sort_comparison_suite, NumericTest};
    use crate::utils::{cdiv, get_tile_counts};
    use crate::RadixSort;
    use rayon::current_num_threads;

    fn test_mt_lsb_sort_adapter<T>(shift: T)
    where
        T: NumericTest<T>,
    {
        sort_comparison_suite(shift, |inputs| {
            if inputs.len() == 0 {
                return;
            }

            let tile_size = cdiv(inputs.len(), current_num_threads());
            mt_lsb_sort_adapter(inputs, 0, T::LEVELS - 1, tile_size);
        });
    }

    #[test]
    pub fn test_u8() {
        test_mt_lsb_sort_adapter(0u8);
    }

    #[test]
    pub fn test_u16() {
        test_mt_lsb_sort_adapter(8u16);
    }

    #[test]
    pub fn test_u32() {
        test_mt_lsb_sort_adapter(16u32);
    }

    #[test]
    pub fn test_u64() {
        test_mt_lsb_sort_adapter(32u64);
    }

    #[test]
    pub fn test_u128() {
        test_mt_lsb_sort_adapter(64u128);
    }

    #[test]
    pub fn test_usize() {
        test_mt_lsb_sort_adapter(32usize);
    }

    #[test]
    pub fn test_sample() {
        let mut data = [9, 8, 7, 6, 5, 4, 3, 2, 1, 0u32];

        data.radix_sort_unstable();

        assert_eq!(data, [0, 1, 2, 3u32, 4, 5, 6, 7, 8, 9]);
    }
}