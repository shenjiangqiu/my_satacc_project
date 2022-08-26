use std::collections::{BTreeMap, VecDeque};

use crate::{
    satacc::MemReqType,
    sim::{InOutPort, SimComponent, SimReciver, SimSender},
};

use super::{
    icnt::IcntMsgWrapper,
    satacc_minisat_task::{ClauseTask, WatcherTask},
    MemReq, SataccStatus,
};

pub struct Watcher {
    pub watcher_task_receiver: SimReciver<WatcherTask>,
    pub clause_icnt_sender: SimSender<IcntMsgWrapper<ClauseTask>>,
    pub cache_mem_icnt_sender: InOutPort<IcntMsgWrapper<MemReq>>,
    pub private_cache_sender: SimSender<IcntMsgWrapper<MemReq>>,
    pub private_cache_receiver: SimReciver<IcntMsgWrapper<MemReq>>,
    pub meta_finished_queue: VecDeque<WatcherTask>,
    pub data_finished_queue: VecDeque<WatcherTask>,
    single_watcher_task_queue: VecDeque<ClauseTask>,
    single_watcher_value_finished_queue: VecDeque<ClauseTask>,
    single_watcher_process_finished_queue: VecDeque<ClauseTask>,
    current_processing_task: Option<(usize, ClauseTask)>,
    total_watchers: usize,
    watcher_pe_id: usize,
    mem_req_id_to_watcher_task: BTreeMap<usize, WatcherTask>,
    mem_req_id_to_clause_task: BTreeMap<usize, ClauseTask>,
    total_ongoing_meta_mem_reqs: usize,
    total_ongoing_data_mem_reqs: usize,
    total_blocker_requests_ongoing: usize,
}

impl Watcher {
    pub fn new(
        watcher_task_receiver: SimReciver<WatcherTask>,
        clause_icnt_sender: SimSender<IcntMsgWrapper<ClauseTask>>,
        cache_mem_icnt_sender: InOutPort<IcntMsgWrapper<MemReq>>,
        private_cache_sender: SimSender<IcntMsgWrapper<MemReq>>,
        private_cache_receiver: SimReciver<IcntMsgWrapper<MemReq>>,
        total_watchers: usize,
        watcher_pe_id: usize,
    ) -> Self {
        Watcher {
            watcher_task_receiver,
            clause_icnt_sender,
            cache_mem_icnt_sender,
            private_cache_sender,
            private_cache_receiver,
            meta_finished_queue: VecDeque::new(),
            data_finished_queue: VecDeque::new(),
            single_watcher_task_queue: VecDeque::new(),
            single_watcher_value_finished_queue: VecDeque::new(),
            single_watcher_process_finished_queue: VecDeque::new(),
            current_processing_task: None,
            total_watchers,
            watcher_pe_id,
            mem_req_id_to_watcher_task: BTreeMap::new(),
            mem_req_id_to_clause_task: BTreeMap::new(),
            total_ongoing_meta_mem_reqs: 0,
            total_ongoing_data_mem_reqs: 0,
            total_blocker_requests_ongoing: 0,
        }
    }
}
#[derive(Debug)]
enum IdleReason {
    NoTask,
    CannotSendL3Cache,
    CannotSendPrivateCache,
    CannotSendClause,
    WaitingL3Ret,
    WaitingL1Ret,
}
impl SimComponent for Watcher {
    type SharedStatus = SataccStatus;
    fn update(&mut self, context: &mut Self::SharedStatus, current_cycle: usize) -> (bool, bool) {
        let mut busy = false;
        let mut updated = false;
        let mut reason = IdleReason::NoTask;
        // first check the new arrived watcher tasks
        if self.total_ongoing_meta_mem_reqs < 256 && self.meta_finished_queue.len() < 256 {
            if let Ok(watcher_task) = self.watcher_task_receiver.recv() {
                busy = true;
                // first read the watcher metadata
                let mem_meta_task = watcher_task.get_meta_data_task(
                    self.total_watchers,
                    context,
                    self.watcher_pe_id,
                );
                let id = mem_meta_task.msg.id;
                match self.cache_mem_icnt_sender.out_port.send(mem_meta_task) {
                    Ok(_) => {
                        tracing::debug!("Watcher Receive task! {current_cycle}");

                        context.statistics.watcher_statistics[self.watcher_pe_id]
                            .total_assignments += 1;
                        self.mem_req_id_to_watcher_task.insert(id, watcher_task);
                        updated = true;
                        self.total_ongoing_meta_mem_reqs += 1;
                    }
                    Err(_) => {
                        // cannot send to cache now
                        tracing::debug!("cannot meta data request send to cache now");
                        self.watcher_task_receiver.ret(watcher_task);
                        reason = IdleReason::CannotSendL3Cache;
                    }
                }
            }
        }
        // then check the tasks that finihed the meta data read
        if self.total_ongoing_data_mem_reqs < 256 && self.data_finished_queue.len() < 256 {
            if let Some(watcher_task) = self.meta_finished_queue.pop_front() {
                busy = true;
                // start to read the watcher data!
                let mem_watcher_task = watcher_task.get_watcher_data_task(
                    self.total_watchers,
                    context,
                    self.watcher_pe_id,
                );
                let id = mem_watcher_task.msg.id;
                match self.cache_mem_icnt_sender.out_port.send(mem_watcher_task) {
                    Ok(_) => {
                        updated = true;

                        self.mem_req_id_to_watcher_task.insert(id, watcher_task);
                        self.total_ongoing_data_mem_reqs += 1;
                    }
                    Err(_mem_watcher_task) => {
                        // cannot send to cache now
                        tracing::debug!("cannot send watcher data request send to cache now");
                        self.meta_finished_queue.push_front(watcher_task);
                        reason = IdleReason::CannotSendL3Cache;
                    }
                }
            }
        }

        // then check the tasks that finished the watcher list read
        if self.single_watcher_task_queue.len() < 256 {
            if let Some(watcher_task) = self.data_finished_queue.pop_front() {
                busy = true;
                updated = true;
                // start to read the watcher data!
                let signale_watcher_tasks = watcher_task.into_sub_single_watcher_task();
                context.statistics.watcher_statistics[self.watcher_pe_id].total_watchers +=
                    signale_watcher_tasks.len();
                self.single_watcher_task_queue.extend(signale_watcher_tasks);
            }
        }

        // then process the single watcher tasks
        if self.total_blocker_requests_ongoing < 256
            && self.single_watcher_value_finished_queue.len() < 256
        {
            if let Some(single_task) = self.single_watcher_task_queue.pop_front() {
                let blocker_req = single_task.get_blocker_req(self.total_watchers, context);
                let addr = blocker_req.addr;
                let mem_id = ((addr >> 6) & ((1 << 3) - 1)) as usize;

                let id = blocker_req.id;
                busy = true;
                match self.cache_mem_icnt_sender.out_port.send(IcntMsgWrapper {
                    msg: blocker_req,
                    mem_target_port: self.total_watchers + mem_id,
                }) {
                    Ok(_) => {
                        updated = true;

                        self.mem_req_id_to_clause_task.insert(id, single_task);
                        self.total_blocker_requests_ongoing += 1;
                    }
                    Err(_blocker_req) => {
                        // cannot send to cache now
                        tracing::debug!("cannot send blocker request send to cache now");
                        self.single_watcher_task_queue.push_front(single_task);
                        reason = IdleReason::CannotSendPrivateCache;
                    }
                }
            }
        }

        // update current processing task
        if self.single_watcher_process_finished_queue.len() < 256 {
            if let Some((finished_cycle, single_task)) = self.current_processing_task.take() {
                // currently processing task, so busy is true
                busy = true;
                updated = true;

                if finished_cycle > current_cycle {
                    // not finished yet
                    self.current_processing_task = Some((finished_cycle, single_task));
                } else {
                    self.single_watcher_process_finished_queue
                        .push_back(single_task);
                }
            }
        }

        // process the watcher
        if self.current_processing_task.is_none() {
            if let Some(single_task) = self.single_watcher_value_finished_queue.pop_front() {
                busy = true;
                updated = true;
                // a watcher need 2 cycle to test if it's time to read the clause
                let process_time = 2;
                self.current_processing_task = Some((current_cycle + process_time, single_task));
            }
        }

        // then send the task to clause unit
        if let Some(single_task) = self.single_watcher_process_finished_queue.pop_front() {
            busy = true;
            // only send to the clause unit when it has to.
            if single_task.have_to_read_clause() {
                match self
                    .clause_icnt_sender
                    .send(single_task.into_push_clause_req(self.total_watchers))
                {
                    Ok(_) => {
                        updated = true;
                        context.statistics.watcher_statistics[self.watcher_pe_id]
                            .total_clauses_sent += 1;
                    }
                    Err(clause_task) => {
                        // cannot send to cache now
                        tracing::debug!("cannot send clause to clause unit now");
                        let clause_task = clause_task.msg;
                        self.single_watcher_process_finished_queue
                            .push_front(clause_task);
                        reason = IdleReason::CannotSendClause;
                    }
                }
            }
        }

        // get the global memory return

        if let Ok(mem_req) = self.cache_mem_icnt_sender.in_port.recv() {
            busy = true;
            match mem_req.msg.req_type {
                MemReqType::WatcherReadMetaData => {
                    updated = true;
                    tracing::debug!("Watcher Receive mem_req! {current_cycle}");
                    self.meta_finished_queue.push_back(
                        self.mem_req_id_to_watcher_task
                            .remove(&mem_req.msg.id)
                            .unwrap(),
                    );
                    self.total_ongoing_meta_mem_reqs -= 1;
                }
                MemReqType::WatcherReadData => {
                    updated = true;
                    tracing::debug!("Watcher Receive mem_req! {current_cycle}");
                    self.data_finished_queue.push_back(
                        self.mem_req_id_to_watcher_task
                            .remove(&mem_req.msg.id)
                            .unwrap(),
                    );
                    self.total_ongoing_data_mem_reqs -= 1;
                }
                MemReqType::WatcherReadBlocker => {
                    self.single_watcher_value_finished_queue.push_back(
                        self.mem_req_id_to_clause_task
                            .remove(&mem_req.msg.id)
                            .unwrap(),
                    );
                }

                _ => unreachable!(),
            }
        }
        // get the private cache return
        if let Ok(_mem_req) = self.private_cache_receiver.recv() {
            // tracing::debug!("Watcher Receive mem_req private! {current_cycle}");
            // self.total_ongoing_private_cache_reqs -= 1;
            // busy = true;
            // updated = true;
            // match mem_req.msg.req_type {
            //     MemReqType::WatcherReadBlocker => {
            //         self.single_watcher_value_finished_queue.push_back(
            //             self.mem_req_id_to_clause_task
            //                 .remove(&mem_req.msg.id)
            //                 .unwrap(),
            //         );
            //     }
            //     _ => unreachable!(),
            // }
            unreachable!();
        }

        match updated {
            true => {
                context.statistics.watcher_statistics[self.watcher_pe_id].busy_cycle += 1;
            }
            false => {
                if !self.mem_req_id_to_clause_task.is_empty() {
                    reason = IdleReason::WaitingL1Ret;
                }
                if !self.mem_req_id_to_watcher_task.is_empty() {
                    reason = IdleReason::WaitingL3Ret;
                }
                context.statistics.watcher_statistics[self.watcher_pe_id].idle_cycle += 1;
                match reason {
                    IdleReason::NoTask => {
                        context.statistics.watcher_statistics[self.watcher_pe_id]
                            .idle_stat
                            .idle_no_task += 1
                    }
                    IdleReason::CannotSendL3Cache => {
                        context.statistics.watcher_statistics[self.watcher_pe_id]
                            .idle_stat
                            .idle_send_l3 += 1
                    }
                    IdleReason::CannotSendPrivateCache => {
                        context.statistics.watcher_statistics[self.watcher_pe_id]
                            .idle_stat
                            .idle_send_l1 += 1
                    }
                    IdleReason::CannotSendClause => {
                        context.statistics.watcher_statistics[self.watcher_pe_id]
                            .idle_stat
                            .idle_send_clause += 1
                    }
                    IdleReason::WaitingL3Ret => {
                        context.statistics.watcher_statistics[self.watcher_pe_id]
                            .idle_stat
                            .idle_wating_l3 += 1
                    }
                    IdleReason::WaitingL1Ret => {
                        context.statistics.watcher_statistics[self.watcher_pe_id]
                            .idle_stat
                            .idle_wating_l1 += 1
                    }
                }
            }
        }
        if busy && !updated {
            tracing::debug!(
                "Watcher is busy! but not updated {current_cycle},idle reason:{reason:?}"
            );
        }
        (busy, updated)
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn watcher_test() {}
}
