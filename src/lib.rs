#[cfg(test)]
extern crate quickcheck;
#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

pub mod cluster;
pub mod node;

mod hasher {
    const HASHER_SEED: u64 = 23492387589239525;

    /// Returns a new XXH3 Hasher initialized with a static seed (instead of a random one)
    /// Hashers created with this function will produce the same results, regardless of the platform, OS,
    /// or runtime considerations
    pub(crate) fn deterministic_hasher() -> impl std::hash::Hasher {
        twox_hash::xxh3::Hash64::with_seed(HASHER_SEED)
    }
}
