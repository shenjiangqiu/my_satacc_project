use serde::{Deserialize, Serialize};

use super::{get_bit_lens, get_set_number_from_addr, AccessResult};

pub struct FastCache {
    pub cache_config: CacheConfig,
    sets: Vec<Set>,
    set_bit_len: u64,
    block_bit_len: u64,
    channel_bit_len: u64,
}
#[derive(Debug, Clone)]
pub struct Set {
    lines: Vec<u64>,
    replace_ptr: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[repr(C)]
pub struct CacheConfig {
    pub sets: u64,
    pub associativity: u64,
    pub block_size: u64,
    pub channels: u64,
    pub alway_hit: bool,
}

impl FastCache {
    pub fn new(cache_config: &CacheConfig) -> Self {
        let sets = vec![
            Set {
                lines: vec![],
                replace_ptr: 0,
            };
            cache_config.sets as usize
        ];
        let set_bit_len = get_bit_lens(cache_config.sets);
        let block_bit_len = get_bit_lens(cache_config.block_size);
        let channel_bit_len = get_bit_lens(cache_config.channels);
        FastCache {
            cache_config: cache_config.clone(),
            sets,
            set_bit_len,
            block_bit_len,
            channel_bit_len,
        }
    }
    pub fn access(&mut self, addr: u64) -> AccessResult {
        let (set_number, tag) = get_set_number_from_addr(
            addr,
            self.set_bit_len,
            self.block_bit_len,
            self.channel_bit_len,
        );
        // todo! always hit
        match self.cache_config.alway_hit {
            true => AccessResult::Hit(tag),
            false => {
                let set = &mut self.sets[set_number as usize];
                for line in &set.lines {
                    if *line == tag {
                        return AccessResult::Hit(tag);
                    }
                }
                // not in the set
                if set.lines.len() < self.cache_config.associativity as usize {
                    set.lines.push(tag);
                } else {
                    set.lines[set.replace_ptr] = tag;
                    set.replace_ptr =
                        (set.replace_ptr + 1) % self.cache_config.associativity as usize;
                }

                AccessResult::Miss(tag)
            }
        }
    }
    pub fn get_set_bit_len(&self) -> u64 {
        self.set_bit_len
    }
    pub fn get_block_bit_len(&self) -> u64 {
        self.block_bit_len
    }
    pub fn get_channel_bit_len(&self) -> u64 {
        self.channel_bit_len
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_cache() {
        let cache_config = CacheConfig {
            sets: 2,
            associativity: 2,
            block_size: 4,
            channels: 1,
            alway_hit: false,
        };
        let mut cache = FastCache::new(&cache_config);
        // the first one is miss and then the later 4 is in a block so it's hit
        assert!(cache.access(0).as_miss().is_some());
        assert!(cache.access(1).as_hit().is_some());
        assert!(cache.access(2).as_hit().is_some());
        assert!(cache.access(3).as_hit().is_some());
        // the next 3 is miss and will full the cache
        assert!(cache.access(4).as_miss().is_some());
        assert!(cache.access(8).as_miss().is_some());
        assert!(cache.access(12).as_miss().is_some());

        // the next one is hit because 0 is not evicted now
        assert!(cache.access(0).as_hit().is_some());
        // the new one will evict the first one
        assert!(cache.access(16).as_miss().is_some());
        // the first one will miss
        assert!(cache.access(0).as_miss().is_some());
    }
}
