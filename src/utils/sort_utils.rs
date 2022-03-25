use crate::RadixKey;
#[cfg(feature = "multi-threaded")]
use rayon::prelude::*;
#[cfg(feature = "multi-threaded")]
use std::sync::mpsc::channel;

#[inline]
pub fn get_prefix_sums(counts: &[usize; 256]) -> [usize; 256] {
    let mut sums = [0usize; 256];

    let mut running_total = 0;
    for (i, c) in counts.iter().enumerate() {
        sums[i] = running_total;
        running_total += c;
    }

    sums
}

#[inline]
pub fn get_end_offsets(counts: &[usize; 256], prefix_sums: &[usize; 256]) -> [usize; 256] {
    let mut end_offsets = [0usize; 256];

    end_offsets[0..255].copy_from_slice(&prefix_sums[1..256]);
    end_offsets[255] = counts[255] + prefix_sums[255];

    end_offsets
}

#[inline]
#[cfg(feature = "multi-threaded")]
pub fn par_get_counts<T>(bucket: &[T], level: usize) -> [usize; 256]
where
    T: RadixKey + Sized + Send + Sync,
{
    #[cfg(feature = "work_profiles")]
    println!("({}) PAR_COUNT", level);

    if bucket.len() < 400_000 {
        return get_counts(bucket, level);
    }

    let threads = rayon::current_num_threads();
    let chunk_divisor = 8;
    let chunk_size = (bucket.len() / threads / chunk_divisor) + 1;
    let chunks = bucket.par_chunks(chunk_size);
    let len = chunks.len();
    let (tx, rx) = channel();
    chunks.for_each_with(tx, |tx, chunk| {
        let counts = get_counts(chunk, level);
        tx.send(counts).unwrap();
    });

    let mut msb_counts = [0usize; 256];

    for _ in 0..len {
        let counts = rx.recv().unwrap();

        for (i, c) in counts.iter().enumerate() {
            msb_counts[i] += *c;
        }
    }

    msb_counts
}

#[inline]
pub fn get_counts<T>(bucket: &[T], level: usize) -> [usize; 256]
where
    T: RadixKey,
{
    #[cfg(feature = "work_profiles")]
    println!("({}) COUNT", level);

    let mut counts_1 = [0usize; 256];
    let mut counts_2 = [0usize; 256];
    let mut counts_3 = [0usize; 256];
    let mut counts_4 = [0usize; 256];
    let chunks = bucket.chunks_exact(4);
    let rem = chunks.remainder();

    chunks.into_iter().for_each(|chunk| {
        let a = chunk[0].get_level(level) as usize;
        let b = chunk[1].get_level(level) as usize;
        let c = chunk[2].get_level(level) as usize;
        let d = chunk[3].get_level(level) as usize;

        counts_1[a] += 1;
        counts_2[b] += 1;
        counts_3[c] += 1;
        counts_4[d] += 1;
    });

    rem.iter().for_each(|v| {
        let b = v.get_level(level) as usize;
        counts_1[b] += 1;
    });

    for i in 0..256 {
        counts_1[i] += counts_2[i];
        counts_1[i] += counts_3[i];
        counts_1[i] += counts_4[i];
    }

    counts_1
}

#[allow(clippy::uninit_vec)]
#[inline]
pub fn get_tmp_bucket<T>(len: usize) -> Vec<T> {
    let mut tmp_bucket = Vec::with_capacity(len);
    unsafe {
        // Safety: This will leave the vec with potentially uninitialized data
        // however as we account for every value when placing things
        // into tmp_bucket, this is "safe". This is used because it provides a
        // very significant speed improvement over resize, to_vec etc.
        tmp_bucket.set_len(len);
    }

    tmp_bucket
}

pub fn detect_plateaus<T>(bucket: &[T], level: usize) -> Vec<(u8, usize, usize)>
where
    T: RadixKey + Sized + Send + Sync,
{
    let plateau_min_size = bucket.len() >> 4;

    // 128 is arbitrarily chosen. For small plateau
    // sizes the overhead of this method outweighs any benefits
    if plateau_min_size < 128 {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    let mut plateaus = Vec::new();

    let mut current = 0;
    let mut start = None;
    let mut end = None;

    // 1. Find candidates by looking for the same radix `plateau_min_size` apart.
    for (i, v) in bucket.iter().enumerate().step_by(plateau_min_size) {
        let b = v.get_level(level);

        if b == current {
            end = Some(i);
        } else {
            if let Some(s) = start {
                if let Some(e) = end {
                    if s != e {
                        candidates.push((current, s, s, e, e));
                    }
                }
            }
            current = b;
            start = Some(i);
            end = Some(i);
        }
    }

    // 2. Explore between candidates to find out if this is a plateau
    for (radix, sl, sr, el, er) in candidates.iter_mut() {
        // 2.1 Explore left of the start
        let mut i = *sl;
        loop {
            if i == 0 {
                break;
            }

            i -= 1;

            let b = bucket[i].get_level(level);

            if b != *radix {
                *sl = i + 1;
                break;
            }
        }

        // 2.2 Explore right of the start
        i = *sr;
        loop {
            if i == bucket.len() - 1 {
                break;
            }

            i += 1;

            let b = bucket[i].get_level(level);
            if b != *radix {
                *sr = i;
                break;
            }
        }

        // 2.3 Check if we have a plateau across both points
        if *sr > *er {
            // This is one big plateau
            plateaus.push((*radix, *sl, *sr));
            continue;
        } else if *sr - *sl >= plateau_min_size {
            // This is still a plateau, just around the start point. It doesn't
            // extend all the way to the end point.
            plateaus.push((*radix, *sl, *sr));
        }

        if *el - *sr < plateau_min_size {
            // This end point cannot be a separate plateau
            continue;
        }

        // There is still a chance that the end point is a plateau, check to the left
        // 2.4 Explore to the left of the end point
        i = *el;
        loop {
            if i == *sr {
                // We should never get this far.
                break;
            }

            i -= 1;

            let b = bucket[i].get_level(level);

            if b != *radix {
                *el = i + 1;
                break;
            }
        }

        // 2.5 Explore to the right of the end point
        i = *er;
        loop {
            if i == bucket.len() - 1 {
                break;
            }

            i += 1;

            let b = bucket[i].get_level(level);
            if b != *radix {
                *er = i;
                break;
            }
        }

        // 2.6 Check if the end point is a separate plateau
        if *er - *el >= plateau_min_size {
            plateaus.push((*radix, *el, *er));
        }
    }

    plateaus
}

pub fn apply_plateaus<T>(
    bucket: &mut [T],
    counts: &[usize; 256],
    plateaus: &[(u8, usize, usize)],
) -> ([usize; 256], [usize; 256])
where
    T: RadixKey + Copy + Sized + Send + Sync,
{
    let mut prefix_sums = get_prefix_sums(counts);
    let end_offsets = get_end_offsets(counts, &prefix_sums);

    for (radix, l, r) in plateaus {
        let len = *r - *l;
        let write_start = prefix_sums[*radix as usize];
        let write_end = write_start + len;

        prefix_sums[*radix as usize] += len;

        if *r == write_start && *l == write_end {
            // This is already in-place
            continue;
        } else if *r < write_start || *l > write_end {
            // This is non-overlapping
            let mut tmp_plateau = get_tmp_bucket(len);
            let mut tmp_destination = get_tmp_bucket(len);

            tmp_plateau.copy_from_slice(&bucket[*l..*r]);
            tmp_destination.copy_from_slice(&bucket[write_start..write_end]);
            bucket[write_start..write_end].copy_from_slice(&tmp_plateau);
            bucket[*l..*r].copy_from_slice(&tmp_destination);
        } else if *r < write_end {
            // The right side of the plateau overlaps with the write area
            // Find the non-overlapping size on either end
            let non_overlapping_len = write_start - *l;
            let mut tmp_plateau = get_tmp_bucket(non_overlapping_len);
            let mut tmp_destination = get_tmp_bucket(non_overlapping_len);
            tmp_plateau.copy_from_slice(&bucket[*l..write_start]);
            tmp_destination.copy_from_slice(&bucket[*r..write_end]);
            bucket[*l..write_start].copy_from_slice(&tmp_destination);
            bucket[*r..write_end].copy_from_slice(&tmp_plateau);
        } else {
            // The left side of the plateau overlaps with the write area
            // Find the non-overlapping size on either end
            let non_overlapping_len = *r - write_end;
            let mut tmp_plateau = get_tmp_bucket(non_overlapping_len);
            let mut tmp_destination = get_tmp_bucket(non_overlapping_len);
            tmp_plateau.copy_from_slice(&bucket[write_end..*r]);
            tmp_destination.copy_from_slice(&bucket[write_start..*l]);
            bucket[write_end..*r].copy_from_slice(&tmp_destination);
            bucket[write_start..*l].copy_from_slice(&tmp_plateau);
        }
    }

    (prefix_sums, end_offsets)
}

#[inline]
pub const fn cdiv(a: usize, b: usize) -> usize {
    (a + b - 1) / b
}

#[inline]
pub fn get_tile_counts<T>(bucket: &[T], tile_size: usize, level: usize) -> Vec<[usize; 256]>
where
    T: RadixKey + Copy + Sized + Send + Sync,
{
    #[cfg(feature = "work_profiles")]
    println!("({}) TILE_COUNT", level);

    #[cfg(feature = "multi-threaded")]
    return bucket
        .par_chunks(tile_size)
        .map(|chunk| par_get_counts(chunk, level))
        .collect();

    #[cfg(not(feature = "multi-threaded"))]
    return bucket
        .chunks(tile_size)
        .map(|chunk| get_counts(chunk, level))
        .collect();
}

#[inline]
pub fn aggregate_tile_counts(tile_counts: &[[usize; 256]]) -> [usize; 256] {
    let mut out = tile_counts[0];
    for tile in tile_counts.iter().skip(1) {
        for i in 0..256 {
            out[i] += tile[i];
        }
    }

    out
}

#[inline]
pub fn is_homogenous_bucket(counts: &[usize; 256]) -> bool {
    let mut seen = false;
    for c in counts {
        if *c > 0 {
            if seen {
                return false;
            } else {
                seen = true;
            }
        }
    }

    true
}
