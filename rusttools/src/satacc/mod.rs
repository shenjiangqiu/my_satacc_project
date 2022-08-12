pub(self) mod cache;
pub(self) mod clause;
pub(self) mod icnt;
pub(self) mod satacc_minisat_task;
pub(self) mod simulator;
pub(self) mod trail;
pub(self) mod watcher;
pub(self) mod watcher_interface;
pub(self) mod wating_task;

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
#[derive(Debug)]
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

pub use satacc_minisat_task::SataccMinisatTask;
pub use simulator::Simulator;

use self::satacc_minisat_task::ClauseTask;
#[derive(Default, Debug)]
pub struct SataccStatus {
    pub current_mem_req_id: usize,
    pub cache_status: cache::CacheStatus,
}

impl SataccStatus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn next_mem_id(&mut self) -> usize {
        self.current_mem_req_id += 1;
        self.current_mem_req_id
    }
}
