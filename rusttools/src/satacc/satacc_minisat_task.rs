use std::collections::VecDeque;

use super::{icnt::IcntMsgWrapper, MemReq, MemReqType, SataccStatus};
/// # SataccMinisatTask
/// the full task of the whole SAT solver
/// - it contains many decisions in [`SingleRoundTask`]
pub struct SataccMinisatTask {
    pub decisions: VecDeque<SingleRoundTask>,
}

/// # SingleRoundTask
/// the task for a single decisions that made by the softwares
/// - it contains many assignments represented by [`WatcherTask`]
///
pub struct SingleRoundTask {
    pub assignments: VecDeque<WatcherTask>,
}

impl SingleRoundTask {
    pub fn pop_next_task(&mut self) -> Option<WatcherTask> {
        self.assignments.pop_front()
    }
    pub fn ret_task(&mut self, task: WatcherTask) {
        self.assignments.push_front(task);
    }
}
#[derive(Default)]
pub struct SingleRoundStatistics {
    pub total_assignments: usize,
    pub total_watchers: usize,
    pub total_clauses: usize,
}
impl SingleRoundTask {
    pub fn get_statistics(&self) -> SingleRoundStatistics {
        let mut stat = SingleRoundStatistics::default();
        for watcher_task in self.assignments.iter() {
            stat.total_assignments += 1;
            for clause_task in watcher_task.single_watcher_tasks.iter() {
                stat.total_watchers += 1;
                if clause_task.clause_data.is_some() {
                    stat.total_clauses += 1;
                }
            }
        }
        stat
    }
}

/// # WatcherTask
/// the task for a single assignment that made by the softwares
/// - a assignment should have a watcher list, it contains many watchers:[`ClauseTask`]
/// - each watcher have a blocker and a Clause task:[`ClauseTask`]
#[derive(Debug, Default)]
pub struct WatcherTask {
    /// the level of the current task
    pub(crate) level: usize,
    /// the watcher list meta data addr
    pub(crate) meta_data_addr: u64,
    /// the watcher list value
    pub(crate) watcher_addr: u64,
    /// the assign literal
    pub(crate) watcher_id: usize,
    /// the time to process the watcher list
    /// the watchers to be processed
    pub(crate) single_watcher_tasks: VecDeque<ClauseTask>,
}

#[derive(Debug)]
pub struct ClauseData {
    pub clause_id: usize,
    pub clause_addr: u64,
    pub clause_processing_time: usize,
    pub clause_value_addr: Vec<u64>,
    pub clause_value_id: Vec<usize>,
}

/// # ClauseTask
/// the single watcher that related to a clause
#[derive(Debug)]
pub struct ClauseTask {
    pub watcher_id: usize,
    pub blocker_addr: u64,
    pub clause_data: Option<ClauseData>,
}
impl ClauseTask {
    pub fn into_push_clause_req(self, total_watchers: usize) -> IcntMsgWrapper<Self> {
        let target_watcher_id = self.get_watcher_pe_id(total_watchers);
        IcntMsgWrapper {
            msg: self,
            mem_target_port: target_watcher_id,
        }
    }
    pub fn get_blocker_req(&self, total_watchers: usize, context: &mut SataccStatus) -> MemReq {
        let blocker_addr = self.blocker_addr;
        MemReq {
            addr: blocker_addr,
            id: context.next_mem_id(),
            watcher_pe_id: self.get_watcher_pe_id(total_watchers),
            mem_id: 0,
            is_write: false,
            req_type: MemReqType::WatcherReadBlocker,
        }
    }
    pub fn have_to_read_clause(&self) -> bool {
        self.clause_data.is_some()
    }
    pub fn get_clause_data_task(
        &self,
        context: &mut SataccStatus,
        watcher_pe_id: usize,
        clause_pe_id: usize,
        total_watchers: usize,
    ) -> IcntMsgWrapper<MemReq> {
        match &self.clause_data {
            Some(clause_data) => {
                let mem_id = ((clause_data.clause_addr >> 6) & ((1 << 3) - 1)) as usize;
                let req = MemReq {
                    addr: clause_data.clause_addr,
                    id: context.next_mem_id(),
                    watcher_pe_id: watcher_pe_id,
                    mem_id,
                    is_write: false,
                    req_type: MemReqType::ClauseReadData(clause_pe_id),
                };
                IcntMsgWrapper {
                    msg: req,
                    mem_target_port: total_watchers + mem_id,
                }
            }
            None => panic!("clause data is none"),
        }
    }
    pub fn get_read_clause_value_task(
        &self,
        context: &mut SataccStatus,
        watcher_pe_id: usize,
        clause_pe_id: usize,
        total_watchers: usize,
    ) -> Vec<IcntMsgWrapper<MemReq>> {
        let clause_data = self.clause_data.as_ref().unwrap();
        let clause_value_data = clause_data.clause_value_addr.clone();

        clause_value_data
            .into_iter()
            .map(|addr| {
                let mem_id = ((addr >> 6) & ((1 << 3) - 1)) as usize;

                let req = MemReq {
                    addr,
                    id: context.next_mem_id(),
                    watcher_pe_id: watcher_pe_id,
                    mem_id,
                    is_write: false,
                    req_type: MemReqType::ClauseReadValue(clause_pe_id),
                };
                IcntMsgWrapper {
                    msg: req,
                    mem_target_port: total_watchers + mem_id,
                }
            })
            .collect()
    }
    pub fn get_watcher_pe_id(&self, total_watchers: usize) -> usize {
        self.watcher_id / 2 % total_watchers
    }
    /// the process time to process the whole clause
    pub fn get_process_time(&self) -> usize {
        self.clause_data.as_ref().unwrap().clause_processing_time
    }
    pub fn get_inner_clause_pe_id(&self, num_clause_per_watcher: usize) -> usize {
        self.clause_data.as_ref().unwrap().clause_id % num_clause_per_watcher
    }
}
impl SataccMinisatTask {
    pub fn new() -> Self {
        Self {
            decisions: VecDeque::new(),
        }
    }

    #[no_mangle]
    /// this will create a simulator task object, do not free it, it will be freed by calling `run_full_expr`
    pub extern "C" fn create_empty_task() -> *mut Self {
        Box::into_raw(Box::new(Self::new()))
    }

    #[no_mangle]
    pub extern "C" fn start_new_assgin(&mut self) {
        self.decisions.push_back(SingleRoundTask {
            assignments: VecDeque::new(),
        });
    }

    #[no_mangle]
    pub extern "C" fn release_task(task: *mut Self) {
        unsafe {
            Box::from_raw(task);
        }
    }

    #[no_mangle]
    pub extern "C" fn add_watcher_task(
        &mut self,
        level: usize,
        meta_data_addr: u64,
        watcher_addr: u64,
        watcher_id: usize,
    ) {
        self.decisions
            .back_mut()
            .unwrap()
            .assignments
            .push_back(WatcherTask {
                level,
                meta_data_addr,
                watcher_addr,
                watcher_id,
                single_watcher_tasks: VecDeque::new(),
            });
    }
    #[no_mangle]
    pub extern "C" fn add_single_watcher_task_no_clause(
        &mut self,
        blocker_addr: u64,
        watcher_id: usize,
    ) {
        self.decisions
            .back_mut()
            .unwrap()
            .assignments
            .back_mut()
            .unwrap()
            .single_watcher_tasks
            .push_back(ClauseTask {
                blocker_addr,
                watcher_id,
                clause_data: None,
            });
    }

    #[no_mangle]
    pub extern "C" fn add_single_watcher_task(
        &mut self,
        blocker_addr: u64,
        clause_addr: u64,
        clause_id: usize,
        processing_time: usize,
        watcher_id: usize,
    ) {
        self.decisions
            .back_mut()
            .unwrap()
            .assignments
            .back_mut()
            .unwrap()
            .single_watcher_tasks
            .push_back(ClauseTask {
                blocker_addr,
                watcher_id,
                clause_data: Some(ClauseData {
                    clause_id,
                    clause_addr,
                    clause_processing_time: processing_time,
                    clause_value_addr: Vec::new(),
                    clause_value_id: Vec::new(),
                }),
            });
    }
    #[no_mangle]
    pub extern "C" fn add_single_watcher_clause_value_addr(
        &mut self,
        value_addr: u64,
        clause_id: usize,
    ) {
        let last_single_watcher = self
            .decisions
            .back_mut()
            .unwrap()
            .assignments
            .back_mut()
            .unwrap()
            .single_watcher_tasks
            .back_mut()
            .unwrap();
        last_single_watcher
            .clause_data
            .as_mut()
            .unwrap()
            .clause_value_addr
            .push(value_addr);
        last_single_watcher
            .clause_data
            .as_mut()
            .unwrap()
            .clause_value_id
            .push(clause_id);
    }
}

impl SataccMinisatTask {
    pub fn pop_next_task(&mut self) -> Option<SingleRoundTask> {
        self.decisions.pop_front()
    }
}
impl WatcherTask {
    pub fn get_watcher_pe_id(&self, total_watchers: usize) -> usize {
        return (self.watcher_id / 2) % total_watchers;
    }
    pub fn get_total_level_tasks(&self) -> usize {
        // the watcher self and all single watchers
        self.single_watcher_tasks.len() + 1
    }
    pub fn get_meta_data_task(
        &self,
        total_watchers: usize,
        context: &mut SataccStatus,
        watcher_pe_id: usize,
    ) -> IcntMsgWrapper<MemReq> {
        let addr = self.meta_data_addr;
        let partion_id = ((addr >> 6) & ((1 << 3) - 1)) as usize;
        IcntMsgWrapper {
            msg: MemReq {
                addr: addr,
                id: context.next_mem_id(),
                mem_id: partion_id,
                is_write: false,
                req_type: MemReqType::WatcherReadMetaData,
                watcher_pe_id,
            },
            mem_target_port: total_watchers + partion_id,
        }
    }
    pub fn get_watcher_data_task(
        &self,
        total_watchers: usize,
        context: &mut SataccStatus,
        watcher_pe_id: usize,
    ) -> IcntMsgWrapper<MemReq> {
        let addr = self.watcher_addr;
        let partion_id = ((addr >> 6) & ((1 << 3) - 1)) as usize;
        IcntMsgWrapper {
            msg: MemReq {
                addr,
                id: context.next_mem_id(),
                mem_id: partion_id,
                is_write: false,
                req_type: MemReqType::WatcherReadData,
                watcher_pe_id,
            },
            mem_target_port: total_watchers + partion_id,
        }
    }
    pub fn into_sub_single_watcher_task(self) -> VecDeque<ClauseTask> {
        self.single_watcher_tasks
    }
}
