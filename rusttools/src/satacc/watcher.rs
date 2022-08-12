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
        }
    }
}

impl SimComponent for Watcher {
    type SharedStatus = SataccStatus;
    fn update(&mut self, context: &mut Self::SharedStatus, current_cycle: usize) -> bool {
        let mut busy = false;
        // first check the new arrived watcher tasks
        if let Ok(watcher_task) = self.watcher_task_receiver.recv() {
            log::debug!("Watcher Receive task! {current_cycle}");
            // first read the watcher metadata
            let mem_meta_task =
                watcher_task.get_meta_data_task(self.total_watchers, context, self.watcher_pe_id);
            let id = mem_meta_task.msg.id;
            match self.cache_mem_icnt_sender.out_port.send(mem_meta_task) {
                Ok(_) => {
                    self.mem_req_id_to_watcher_task.insert(id, watcher_task);
                    busy = true;
                }
                Err(_) => {
                    // cannot send to cache now
                    self.watcher_task_receiver.ret(watcher_task);
                }
            }
        }
        // then check the tasks that finihed the meta data read
        if let Some(watcher_task) = self.meta_finished_queue.pop_front() {
            // start to read the watcher data!
            let mem_watcher_task = watcher_task.get_watcher_data_task(
                self.total_watchers,
                context,
                self.watcher_pe_id,
            );
            let id = mem_watcher_task.msg.id;
            match self.cache_mem_icnt_sender.out_port.send(mem_watcher_task) {
                Ok(_) => {
                    self.mem_req_id_to_watcher_task.insert(id, watcher_task);
                    busy = true;
                }
                Err(_mem_watcher_task) => {
                    // cannot send to cache now
                    self.meta_finished_queue.push_front(watcher_task);
                }
            }
        }
        // then check the tasks that finished the watcher list read
        if let Some(watcher_task) = self.data_finished_queue.pop_front() {
            busy = true;
            // start to read the watcher data!
            let signale_watcher_tasks = watcher_task.into_sub_single_watcher_task();
            self.single_watcher_task_queue.extend(signale_watcher_tasks);
        }
        // then process the single watcher tasks
        if let Some(single_task) = self.single_watcher_task_queue.pop_front() {
            let blocker_req = single_task.get_blocker_req(self.total_watchers, context);
            let id = blocker_req.id;
            match self.private_cache_sender.send(IcntMsgWrapper {
                msg: blocker_req,
                mem_target_port: 0,
            }) {
                Ok(_) => {
                    busy = true;
                    self.mem_req_id_to_clause_task.insert(id, single_task);
                }
                Err(_blocker_req) => {
                    // cannot send to cache now
                    self.single_watcher_task_queue.push_front(single_task);
                }
            }
        }

        // update current processing task
        if let Some((finished_cycle, single_task)) = self.current_processing_task.take() {
            busy = true;
            if finished_cycle > current_cycle {
                self.current_processing_task = Some((finished_cycle, single_task));
            } else {
                self.single_watcher_process_finished_queue
                    .push_back(single_task);
            }
        }

        // process the watcher
        if self.current_processing_task.is_none() {
            if let Some(single_task) = self.single_watcher_value_finished_queue.pop_front() {
                busy = true;
                // a watcher need 2 cycle to test if it's time to read the clause
                let process_time = 2;
                self.current_processing_task = Some((current_cycle + process_time, single_task));
            }
        }

        // then send the task to clause unit
        if let Some(single_task) = self.single_watcher_process_finished_queue.pop_front() {
            // only send to the clause unit when it has to.
            if single_task.have_to_read_clause() {
                match self
                    .clause_icnt_sender
                    .send(single_task.into_push_clause_req(self.total_watchers))
                {
                    Ok(_) => busy = true,
                    Err(clause_task) => {
                        // cannot send to cache now
                        let clause_task = clause_task.msg;
                        self.single_watcher_process_finished_queue
                            .push_front(clause_task);
                    }
                }
            }
        }

        // get the global memory return
        if let Ok(mem_req) = self.cache_mem_icnt_sender.in_port.recv() {
            busy = true;
            log::debug!("Watcher Receive mem_req! {current_cycle}");
            match mem_req.msg.req_type {
                MemReqType::WatcherReadMetaData => {
                    self.meta_finished_queue.push_back(
                        self.mem_req_id_to_watcher_task
                            .remove(&mem_req.msg.id)
                            .unwrap(),
                    );
                }
                MemReqType::WatcherReadData => {
                    self.data_finished_queue.push_back(
                        self.mem_req_id_to_watcher_task
                            .remove(&mem_req.msg.id)
                            .unwrap(),
                    );
                }

                _ => unreachable!(),
            }
        }
        // get the private cache return
        if let Ok(mem_req) = self.private_cache_receiver.recv() {
            log::debug!("Watcher Receive mem_req! {current_cycle}");
            busy = true;
            match mem_req.msg.req_type {
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
        busy
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn watcher_test() {}
}
