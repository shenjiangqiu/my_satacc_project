use eyre::{Context, Result};
use libc::c_char;
use ramulator_wrapper::PresetConfigs;
use std::{ffi::CStr, fs};

use serde::{Deserialize, Serialize};

use crate::satacc::{simulator::RunMode, CacheConfig};

/// The type for the watcher sending to the clase
#[repr(C)]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum WatcherToClauseType {
    /// - in this case, the watcher send clause to it's own clause unit
    /// - so the clause should use icnt to send memory request
    Streight,
    /// - in this case, the watcher send clause to dedicate clause unit
    /// - so the clause should direct to send memory request
    Icnt,
}
/// the type of the dram that used to read and write data
#[repr(C)]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum DramType {
    DDR4,
    HBM,
}

#[repr(C)]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum IcntType {
    Mesh,
    Ring,
    Ideal,
}
#[derive(Debug, Deserialize, Serialize, Clone)]
#[repr(C)]
pub enum CacheType {
    Simple,
    Ramu,
}
/// the config for satacc
///
#[repr(C)]
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    pub watcher_to_clause_type: WatcherToClauseType,
    pub n_watchers: usize,
    /// the number of clause unit per watcher
    pub n_clauses: usize,
    pub mems: usize,
    pub icnt: IcntType,
    pub seq: bool,
    pub ideal_memory: bool,
    pub ideal_l3cache: bool,
    pub multi_port: usize,
    pub dram_config: DramType,
    pub watcher_to_clause_icnt: IcntType,
    pub watcher_to_writer_icnt: IcntType,
    pub num_writer_entry: usize,
    pub num_writer_merge: usize,
    pub single_watcher: bool,
    pub private_cache_size: usize,
    pub l3_cache_size: usize,
    pub channel_size: usize,
    pub l3_cache_type: CacheType,
    pub ramu_cache_config: PresetConfigs,
    pub l1_hit_latency: usize,
    pub l3_hit_latency: usize,
    pub miss_latency: usize,
    pub init_running_mode: RunMode,
    pub private_cache_config: CacheConfig,
    pub l3_cache_config: CacheConfig,
}
impl Default for Config {
    /// build a default config with 16x16 pes, 1k private cache ,16M l3 cache
    /// the cache is a fixed latency cache
    fn default() -> Self {
        let config = Config {
            watcher_to_clause_type: WatcherToClauseType::Icnt,
            n_watchers: 16,
            n_clauses: 1,
            mems: 8,
            icnt: IcntType::Mesh,
            seq: false,
            ideal_memory: false,
            ideal_l3cache: false,
            multi_port: 1,
            dram_config: DramType::HBM,
            watcher_to_clause_icnt: IcntType::Mesh,
            watcher_to_writer_icnt: IcntType::Mesh,
            num_writer_entry: 1,
            num_writer_merge: 1,
            single_watcher: false,
            private_cache_size: 1,
            l3_cache_size: 1,
            channel_size: 16,
            l3_cache_type: CacheType::Simple,
            init_running_mode: RunMode::RealRoundGap,
            ramu_cache_config: ramulator_wrapper::PresetConfigs::HBM,
            private_cache_config: CacheConfig {
                sets: 4,
                associativity: 4,
                block_size: 64,
                channels: 1,
            },
            l3_cache_config: CacheConfig {
                sets: 65536,
                associativity: 4,
                block_size: 64,
                channels: 8,
            },
            l1_hit_latency: 1,
            l3_hit_latency: 15,
            miss_latency: 120,
        };
        config
    }
}
impl Config {
    pub fn from_config_file(config_file: &str) -> Result<Config> {
        let config_file = fs::read_to_string(config_file).wrap_err("cannot read config file")?;
        let config: Config =
            toml::from_str(&config_file).wrap_err("cannot deserialize to Config")?;
        Ok(config)
    }

    #[no_mangle]
    pub extern "C" fn show_config(&self) {
        println!("{}", serde_json::to_string_pretty(self).unwrap());
    }

    #[no_mangle]
    pub extern "C" fn config_from_file(path: *const c_char) -> Config {
        let config_file = unsafe {
            CStr::from_ptr(path as *const _)
                .to_str()
                .expect("invalide path!")
        };

        Self::from_config_file(config_file).expect("cannot read config file")
    }
}

#[cfg(test)]
mod test {
    use std::fs;

    use super::Config;

    #[test]
    #[ignore]
    fn test_generate_config_file() {
        let config = Config::default();
        let config_file = "satacc_config_sample.toml";
        let content = toml::to_string_pretty(&config).unwrap();
        fs::write(config_file, content).unwrap();
    }
}
