pub(self) mod cache;
pub(self) mod clause;
pub(self) mod icnt;
pub(self) mod satacc_minisat_task;
pub mod simulator;
pub(self) mod statistics;
pub(self) mod trail;
pub(self) mod watcher;
pub(self) mod watcher_interface;
pub(self) mod wating_task;
use std::fs::File;

pub use cache::CacheConfig;
#[derive(Debug)]
pub enum WatcherAccessType {
    ReadMeta,
    ReadData,
}
#[derive(Debug)]
pub enum ClauseAccessType {
    ReadClause(ClauseTask),
    ReadValue,
}
#[derive(Debug, EnumAsInner)]
pub enum MemReqType {
    ClauseReadData(usize),
    ClauseReadValue(usize),
    WatcherReadMetaData,
    WatcherReadData,
    WatcherReadBlocker,
}

#[derive(Debug)]
pub struct MemReq {
    pub addr: u64,
    pub id: usize,
    pub watcher_pe_id: usize,
    pub mem_id: usize,
    pub is_write: bool,
    pub req_type: MemReqType,
}

use enum_as_inner::EnumAsInner;
pub use satacc_minisat_task::SataccMinisatTask;
pub use simulator::Simulator;

use crate::config::Config;

use self::{
    satacc_minisat_task::{ClauseTask, SingleRoundTask},
    statistics::Statistics,
};
#[derive(Default, Debug)]
pub struct SataccStatus {
    pub current_mem_req_id: usize,
    pub statistics: Statistics,
    pub verbose_mode: bool,
    pub current_level_finished_tasks: usize,
}

impl SataccStatus {
    pub fn new(config: Config) -> Self {
        let statistics = Statistics::new(config);
        SataccStatus {
            current_mem_req_id: 0,
            statistics,
            verbose_mode: false,
            current_level_finished_tasks: 0,
        }
    }

    pub fn next_mem_id(&mut self) -> usize {
        self.current_mem_req_id += 1;
        self.current_mem_req_id
    }
    /// update each round's statistics
    pub fn update_single_round_task(&mut self, single_round_task: &SingleRoundTask) {
        self.statistics.update_single_round_task(single_round_task);
    }

    pub fn save_statistics(&self, path: &str) {
        serde_json::to_writer_pretty(File::create(path).unwrap(), &self.statistics).unwrap();
    }
}
