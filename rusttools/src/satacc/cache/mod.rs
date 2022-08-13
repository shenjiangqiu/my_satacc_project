#[derive(Default, Debug)]
pub struct CacheStatus {
    pub hits: usize,
    pub misses: usize,
}
pub enum CacheId {
    L3Cache,
    PrivateCache(usize),
}
/// Get the len in bits of a u32 int.
pub(self) fn get_bit_lens(size: u64) -> u64 {
    let mut len: u64 = 0;
    let mut temp = size;
    while temp > 1 {
        temp /= 2;
        len += 1;
    }
    len
}
/// return the index of the set and the tag
pub(self) fn get_set_number_from_addr(
    addr: u64,
    set_bit_len: u64,
    block_bit_line: u64,
    _channel_bit_len: u64,
) -> (u64, u64) {
    let set_number = (addr >> block_bit_line) & ((1 << set_bit_len) - 1);
    let tag = addr & !((1 << block_bit_line) - 1);
    (set_number, tag)
}

mod cache_with_fix_time;
mod cache_with_ramulator;
mod fast_cache;
#[derive(Debug, EnumAsInner)]
pub enum AccessResult {
    Hit(u64),
    Miss(u64),
}
pub use cache_with_fix_time::CacheWithFixTime;
pub use cache_with_ramulator::CacheWithRamulator;
use enum_as_inner::EnumAsInner;
pub use fast_cache::CacheConfig;
pub use fast_cache::FastCache;
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_bit_lens() {
        assert_eq!(get_bit_lens(2), 1);
        assert_eq!(get_bit_lens(4), 2);
        assert_eq!(get_bit_lens(8), 3);
    }
}
