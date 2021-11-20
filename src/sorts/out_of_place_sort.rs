use crate::utils::*;
use crate::RadixKey;

#[inline]
pub fn out_of_place_sort<T>(
    src_bucket: &[T],
    dst_bucket: &mut [T],
    counts: &[usize; 256],
    level: usize,
) where
    T: RadixKey + Sized + Send + Copy + Sync,
{
    let mut prefix_sums = get_prefix_sums(counts);

    let chunks = src_bucket.chunks_exact(8);
    let rem = chunks.remainder();

    chunks.into_iter().for_each(|chunk| {
        let a = chunk[0].get_level(level) as usize;
        let b = chunk[1].get_level(level) as usize;
        let c = chunk[2].get_level(level) as usize;
        let d = chunk[3].get_level(level) as usize;
        let e = chunk[4].get_level(level) as usize;
        let f = chunk[5].get_level(level) as usize;
        let g = chunk[6].get_level(level) as usize;
        let h = chunk[7].get_level(level) as usize;

        dst_bucket[prefix_sums[a]] = chunk[0];
        prefix_sums[a] += 1;
        dst_bucket[prefix_sums[b]] = chunk[1];
        prefix_sums[b] += 1;
        dst_bucket[prefix_sums[c]] = chunk[2];
        prefix_sums[c] += 1;
        dst_bucket[prefix_sums[d]] = chunk[3];
        prefix_sums[d] += 1;
        dst_bucket[prefix_sums[e]] = chunk[4];
        prefix_sums[e] += 1;
        dst_bucket[prefix_sums[f]] = chunk[5];
        prefix_sums[f] += 1;
        dst_bucket[prefix_sums[g]] = chunk[6];
        prefix_sums[g] += 1;
        dst_bucket[prefix_sums[h]] = chunk[7];
        prefix_sums[h] += 1;
    });

    rem.iter().for_each(|val| {
        let b = val.get_level(level) as usize;
        dst_bucket[prefix_sums[b]] = *val;
        prefix_sums[b] += 1;
    });
}

#[inline]
pub fn out_of_place_sort_with_counts<T>(
    src_bucket: &[T],
    dst_bucket: &mut [T],
    counts: &[usize; 256],
    level: usize,
) -> [usize; 256]
where
    T: RadixKey + Sized + Send + Copy + Sync,
{
    let next_level = level + 1;
    let mut prefix_sums = get_prefix_sums(counts);
    let mut next_counts_0 = [0usize; 256];
    let mut next_counts_1 = [0usize; 256];

    let chunks = src_bucket.chunks_exact(8);
    let rem = chunks.remainder();

    chunks.into_iter().for_each(|chunk| {
        let b0 = chunk[0].get_level(level) as usize;
        let bn0 = chunk[0].get_level(next_level) as usize;
        let b1 = chunk[1].get_level(level) as usize;
        let bn1 = chunk[1].get_level(next_level) as usize;
        let b2 = chunk[2].get_level(level) as usize;
        let bn2 = chunk[2].get_level(next_level) as usize;
        let b3 = chunk[3].get_level(level) as usize;
        let bn3 = chunk[3].get_level(next_level) as usize;
        let b4 = chunk[4].get_level(level) as usize;
        let bn4 = chunk[4].get_level(next_level) as usize;
        let b5 = chunk[5].get_level(level) as usize;
        let bn5 = chunk[5].get_level(next_level) as usize;
        let b6 = chunk[6].get_level(level) as usize;
        let bn6 = chunk[6].get_level(next_level) as usize;
        let b7 = chunk[7].get_level(level) as usize;
        let bn7 = chunk[7].get_level(next_level) as usize;

        dst_bucket[prefix_sums[b0]] = chunk[0];
        prefix_sums[b0] += 1;
        next_counts_0[bn0] += 1;
        dst_bucket[prefix_sums[b1]] = chunk[1];
        prefix_sums[b1] += 1;
        next_counts_1[bn1] += 1;
        dst_bucket[prefix_sums[b2]] = chunk[2];
        prefix_sums[b2] += 1;
        next_counts_0[bn2] += 1;
        dst_bucket[prefix_sums[b3]] = chunk[3];
        prefix_sums[b3] += 1;
        next_counts_1[bn3] += 1;
        dst_bucket[prefix_sums[b4]] = chunk[4];
        prefix_sums[b4] += 1;
        next_counts_0[bn4] += 1;
        dst_bucket[prefix_sums[b5]] = chunk[5];
        prefix_sums[b5] += 1;
        next_counts_1[bn5] += 1;
        dst_bucket[prefix_sums[b6]] = chunk[6];
        prefix_sums[b6] += 1;
        next_counts_0[bn6] += 1;
        dst_bucket[prefix_sums[b7]] = chunk[7];
        prefix_sums[b7] += 1;
        next_counts_1[bn7] += 1;
    });

    rem.iter().for_each(|val| {
        let b = val.get_level(level) as usize;
        let bn = val.get_level(next_level) as usize;
        dst_bucket[prefix_sums[b]] = *val;
        prefix_sums[b] += 1;
        next_counts_0[bn] += 1;
    });

    for i in 0..256 {
        next_counts_0[i] += next_counts_1[i];
    }

    next_counts_0
}
