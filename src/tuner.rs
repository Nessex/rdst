use crate::tuner::Algorithm::MtLsbSort;

#[derive(Clone)]
pub struct TuningParams {
    pub threads: usize,
    pub level: usize,
    pub total_levels: usize,
    pub input_len: usize,
    pub parent_len: usize,
    pub in_place: bool,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Algorithm {
    MtOopSort,
    MtLsbSort,
    ScanningSort,
    RecombinatingSort,
    ComparativeSort,
    LrLsbSort,
    LsbSort,
    RegionsSort,
    SkaSort,
}

fn pick_algorithm_standard(p: &TuningParams, counts: &[usize]) -> Algorithm {
    if p.input_len <= 128 {
        return Algorithm::ComparativeSort;
    }

    let depth = p.total_levels - p.level - 1;

    if p.input_len >= 5_000 {
        let distribution_threshold = (p.input_len / 256) * 2;

        // Distribution occurs when the input to be sorted has counts significantly
        // larger than the others
        for c in counts {
            if *c >= distribution_threshold {
                return if depth == 0 {
                    match p.input_len {
                        0..=200_000 => Algorithm::LrLsbSort,
                        200_001..=350_000 => Algorithm::SkaSort,
                        350_001..=4_000_000 => MtLsbSort,
                        4_000_001..=usize::MAX => Algorithm::RegionsSort,
                        _ => Algorithm::LrLsbSort,
                    }
                } else {
                    match p.input_len {
                        0..=200_000 => Algorithm::LrLsbSort,
                        200_001..=800_000 => Algorithm::SkaSort,
                        800_001..=5_000_000 => Algorithm::RecombinatingSort,
                        5_000_001..=usize::MAX => Algorithm::RegionsSort,
                        _ => Algorithm::LrLsbSort,
                    }
                };
            }
        }
    }

    if depth > 0 {
        match p.input_len {
            0..=200_000 => Algorithm::LsbSort,
            200_001..=800_000 => Algorithm::SkaSort,
            800_001..=50_000_000 => Algorithm::RecombinatingSort,
            50_000_001..=usize::MAX => Algorithm::ScanningSort,
            _ => Algorithm::LsbSort,
        }
    } else {
        match p.input_len {
            0..=150_000 => Algorithm::LsbSort,
            150_001..=260_000 => Algorithm::SkaSort,
            260_001..=50_000_000 => Algorithm::RecombinatingSort,
            50_000_001..=usize::MAX => Algorithm::ScanningSort,
            _ => Algorithm::LsbSort,
        }
    }
}

fn pick_algorithm_in_place(p: &TuningParams, counts: &[usize]) -> Algorithm {
    if p.input_len <= 128 {
        return Algorithm::ComparativeSort;
    }

    let depth = p.total_levels - p.level - 1;

    if p.input_len >= 5_000 {
        let distribution_threshold = (p.input_len / 256) * 2;

        // Distribution occurs when the input to be sorted has counts significantly
        // larger than the others
        for c in counts {
            if *c >= distribution_threshold {
                return if depth == 0 {
                    match p.input_len {
                        0..=50_000 => Algorithm::LrLsbSort,
                        50_001..=1_000_000 => Algorithm::SkaSort,
                        1_000_001..=usize::MAX => Algorithm::RegionsSort,
                        _ => Algorithm::LsbSort,
                    }
                } else {
                    match p.input_len {
                        0..=50_000 => Algorithm::LrLsbSort,
                        50_001..=1_000_000 => Algorithm::SkaSort,
                        1_000_001..=usize::MAX => Algorithm::RegionsSort,
                        _ => Algorithm::LsbSort,
                    }
                };
            }
        }
    }

    if depth == 0 {
        match p.input_len {
            0..=50_000 => Algorithm::LsbSort,
            50_001..=1_000_000 => Algorithm::SkaSort,
            1_000_001..=usize::MAX => Algorithm::RegionsSort,
            _ => Algorithm::LsbSort,
        }
    } else {
        match p.input_len {
            0..=50_000 => Algorithm::LsbSort,
            50_001..=1_000_000 => Algorithm::SkaSort,
            1_000_001..=usize::MAX => Algorithm::RegionsSort,
            _ => Algorithm::LsbSort,
        }
    }
}

pub trait Tuner {
    #[inline]
    fn pick_algorithm(&self, p: &TuningParams, counts: &[usize]) -> Algorithm {
        if p.in_place {
            pick_algorithm_in_place(p, counts)
        } else {
            pick_algorithm_standard(p, counts)
        }
    }
}

pub struct DefaultTuner {}
impl Tuner for DefaultTuner {}
