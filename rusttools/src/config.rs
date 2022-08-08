use eyre::{Context, Result};
use libc::c_char;
use std::{ffi::CStr, fs};

use serde::{Deserialize, Serialize};

/// The type for the watcher sending to the clase
#[repr(C)]
#[derive(Debug, Deserialize, Serialize)]
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
#[derive(Debug, Deserialize, Serialize)]
pub enum DramType {
    DDR4,
    HBM,
}

#[repr(C)]
#[derive(Debug, Deserialize, Serialize)]
pub enum IcntType {
    Mesh,
    Ring,
    Ideal,
}
/// the config for satacc
///
#[repr(C)]
#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    watcher_to_clause_type: WatcherToClauseType,
    n_watchers: usize,
    n_clauses: usize,
    mems: usize,
    icnt: IcntType,
    seq: bool,
    ideal_memory: bool,
    ideal_l3cache: bool,
    multi_port: usize,
    dram_config: DramType,
    watcher_to_clause_icnt: IcntType,
    watcher_to_writer_icnt: IcntType,
    num_writer_entry: usize,
    num_writer_merge: usize,
    single_watcher: bool,
    private_cache_size: usize,
    l3_cache_size: usize,
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
