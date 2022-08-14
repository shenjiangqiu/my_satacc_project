use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::{
    satacc::MemReqType,
    sim::{InOutPort, SimComponent, SimReciver},
};

use super::{icnt::IcntMsgWrapper, satacc_minisat_task::ClauseTask, MemReq, SataccStatus};

struct ClauseValueTracker {
    clause_task: ClauseTask,
    waiting_to_send_reqs: VecDeque<IcntMsgWrapper<MemReq>>,
    unfinished_req_id: BTreeSet<usize>,
}
pub struct ClauseUnit {
    watcher_pe_id: usize,
    total_watchers: usize,
    clause_pe_id: usize,
    clause_task_in: SimReciver<IcntMsgWrapper<ClauseTask>>,
    mem_icnt_port: InOutPort<IcntMsgWrapper<MemReq>>,
    private_cache_port: InOutPort<IcntMsgWrapper<MemReq>>,
    clause_data_ready_queue: VecDeque<ClauseTask>,
    clause_value_ready_queue: VecDeque<ClauseTask>,
    current_processing_task: Option<(usize, ClauseTask)>,
    current_reading_value_task: Option<ClauseValueTracker>,

    mem_req_id_to_clause_task: BTreeMap<usize, ClauseTask>,
}
enum BusyReason {
    NoTask,
    WaitingL1,
    WaitingL3,
    SendingL1,
    SendingL3,
}
impl ClauseUnit {
    pub fn new(
        clause_task_in: SimReciver<IcntMsgWrapper<ClauseTask>>,
        mem_icnt_port: InOutPort<IcntMsgWrapper<MemReq>>,
        private_cache_port: InOutPort<IcntMsgWrapper<MemReq>>,
        watcher_pe_id: usize,
        total_watchers: usize,
        clause_pe_id: usize,
    ) -> Self {
        ClauseUnit {
            clause_task_in,
            mem_icnt_port,
            private_cache_port,
            clause_data_ready_queue: VecDeque::new(),
            clause_value_ready_queue: VecDeque::new(),
            current_processing_task: None,
            watcher_pe_id,
            total_watchers,
            clause_pe_id,
            current_reading_value_task: None,
            mem_req_id_to_clause_task: BTreeMap::new(),
        }
    }
}

impl SimComponent for ClauseUnit {
    type SharedStatus = SataccStatus;
    fn update(&mut self, context: &mut Self::SharedStatus, current_cycle: usize) -> bool {
        let mut busy = self.current_processing_task.is_some();
        // first read the clause data
        let mut busy_reason = BusyReason::NoTask;
        if let Ok(task) = self.clause_task_in.recv() {
            log::debug!("ClauseUnit Receive task! {current_cycle}");
            let mem_req = task.msg.get_clause_data_task(
                context,
                self.watcher_pe_id,
                self.clause_pe_id,
                self.total_watchers,
            );
            let id = mem_req.msg.id;
            match self.mem_icnt_port.out_port.send(mem_req) {
                Ok(_) => {
                    self.mem_req_id_to_clause_task.insert(id, task.msg);
                    context.statistics.clause_statistics[self.watcher_pe_id].single_clause
                        [self.clause_pe_id]
                        .total_clause_received += 1;
                    busy = true;
                }
                Err(_e) => {
                    // cannot send to cache now
                    // just ret the task so we don't need the target port
                    self.clause_task_in.ret(task);
                    busy_reason = BusyReason::SendingL3;
                }
            }
        }
        // try get a task to start to read the clause value
        if self.current_reading_value_task.is_none() {
            if let Some(task) = self.clause_data_ready_queue.pop_front() {
                busy = true;
                let mem_req =
                    task.get_read_clause_value_task(context, self.watcher_pe_id, self.clause_pe_id);
                context.statistics.clause_statistics[self.watcher_pe_id].single_clause
                    [self.clause_pe_id]
                    .total_value_read += task.clause_data.as_ref().unwrap().clause_value_addr.len();
                self.current_reading_value_task = Some(ClauseValueTracker {
                    clause_task: task,
                    waiting_to_send_reqs: mem_req.into_iter().collect(),
                    unfinished_req_id: BTreeSet::new(),
                });
            }
        }
        // process the clause value task
        if let Some(reqs) = self.current_reading_value_task.as_mut() {
            if let Some(req) = reqs.waiting_to_send_reqs.pop_front() {
                let id = req.msg.id;
                match self.private_cache_port.out_port.send(req) {
                    Ok(_) => {
                        busy = true;
                        reqs.unfinished_req_id.insert(id);
                    }
                    Err(e) => {
                        // cannot send to cache now
                        // just ret the task so we don't need the target port
                        reqs.waiting_to_send_reqs.push_front(e);
                        busy_reason = BusyReason::SendingL1;
                    }
                }
            }
        }

        // then update current process task
        if let Some((finished_cycle, task)) = self.current_processing_task.take() {
            busy = true;
            if finished_cycle >= current_cycle {
                self.current_processing_task = Some((finished_cycle, task));
            } else {
                // finished
                log::debug!("ClauseUnit finished task! {current_cycle}");
            }
        }
        // then process the value ready task
        if self.current_processing_task.is_none() {
            if let Some(task) = self.clause_value_ready_queue.pop_front() {
                let process_time = task.get_process_time();
                busy = true;
                self.current_processing_task = Some((current_cycle + process_time, task));
            }
        }
        // process memory ret
        if let Ok(mem_req) = self.mem_icnt_port.in_port.recv() {
            log::debug!("ClauseUnit Receive mem_req! {current_cycle}");
            match mem_req.msg.req_type {
                crate::satacc::MemReqType::ClauseReadData(_) => {
                    let clause_task = self
                        .mem_req_id_to_clause_task
                        .remove(&mem_req.msg.id)
                        .unwrap();

                    self.clause_data_ready_queue.push_back(clause_task);
                }
                _ => unreachable!(),
            }
            busy = true;
        }
        // process private cache ret
        if let Ok(mem_req) = self.private_cache_port.in_port.recv() {
            log::debug!("ClauseUnit Receive mem_req from private cache! {current_cycle}");
            match mem_req.msg.req_type {
                MemReqType::ClauseReadValue(_clause_id) => {
                    let current_waiting = self.current_reading_value_task.as_mut().unwrap();
                    current_waiting.unfinished_req_id.remove(&mem_req.msg.id);
                    if current_waiting.unfinished_req_id.is_empty() {
                        let current_waiting = self.current_reading_value_task.take().unwrap();
                        self.clause_value_ready_queue
                            .push_back(current_waiting.clause_task);
                    }
                }
                _ => unreachable!(),
            }
            busy = true;
        }
        match busy {
            true => {
                context.statistics.clause_statistics[self.watcher_pe_id].single_clause
                    [self.clause_pe_id]
                    .busy_cycle += 1;
            }
            false => {
                context.statistics.clause_statistics[self.watcher_pe_id].single_clause
                    [self.clause_pe_id]
                    .idle_cycle += 1;
                // some l3 req in flight
                if !self.mem_req_id_to_clause_task.is_empty() {
                    busy_reason = BusyReason::WaitingL3;
                }
                // some private cache inflight
                if let Some(tracker) = &self.current_reading_value_task {
                    if !tracker.unfinished_req_id.is_empty() {
                        busy_reason = BusyReason::WaitingL1;
                    }
                }
                let idle_stat = &mut context.statistics.clause_statistics[self.watcher_pe_id]
                    .single_clause[self.clause_pe_id]
                    .idle_stat;
                match busy_reason {
                    BusyReason::NoTask => idle_stat.idle_no_task += 1,
                    BusyReason::WaitingL1 => idle_stat.idle_wating_l1 += 1,
                    BusyReason::WaitingL3 => idle_stat.idle_wating_l3 += 1,
                    BusyReason::SendingL1 => idle_stat.idle_send_l1 += 1,
                    BusyReason::SendingL3 => idle_stat.idle_send_l3 += 1,
                }
            }
        }
        busy
    }
}
#[cfg(test)]
mod test {
    use crate::{
        config::Config,
        satacc::{
            icnt::IcntMsgWrapper,
            satacc_minisat_task::{ClauseData, ClauseTask},
            SataccStatus,
        },
        sim::{ChannelBuilder, SimRunner},
        test_utils,
    };

    use super::ClauseUnit;

    #[test]
    fn test_clause_unit() {
        test_utils::init();
        let channel_builder = ChannelBuilder::new();
        let clause_task_port = channel_builder.sim_channel(10);
        let clause_task_in = clause_task_port.1;
        let mem_icnt_port_pair = channel_builder.in_out_port(10);
        let mem_icnt_port = mem_icnt_port_pair.0;
        let private_cache_port_pair = channel_builder.in_out_port(10);
        let private_cache_port = private_cache_port_pair.0;
        let cluase_unit =
            ClauseUnit::new(clause_task_in, mem_icnt_port, private_cache_port, 0, 1, 0);
        let config = Config::default();
        let context = SataccStatus::new(config);
        let mut sim_runner = SimRunner::new(cluase_unit, context);
        clause_task_port
            .0
            .send(IcntMsgWrapper {
                msg: ClauseTask {
                    watcher_id: 1,
                    blocker_addr: 0,
                    clause_data: Some(ClauseData {
                        clause_id: 0,
                        clause_addr: 0,
                        clause_processing_time: 1,
                        clause_value_addr: vec![1, 2, 3],
                        clause_value_id: vec![1, 2, 3],
                    }),
                },
                mem_target_port: 0,
            })
            .unwrap();
        sim_runner.run();
        let req = mem_icnt_port_pair.1.in_port.recv().unwrap();
        log::debug!("should be read data: {:?}", req);
        mem_icnt_port_pair.1.out_port.send(req).unwrap();
        sim_runner.run();
        // now private cache should receive 3 requests
        let req1 = private_cache_port_pair.1.in_port.recv().unwrap();
        let req2 = private_cache_port_pair.1.in_port.recv().unwrap();
        let req3 = private_cache_port_pair.1.in_port.recv().unwrap();
        log::debug!("should be read value: {:?} {:?} {:?}", req1, req2, req3);
        private_cache_port_pair.1.out_port.send(req1).unwrap();
        private_cache_port_pair.1.out_port.send(req2).unwrap();
        private_cache_port_pair.1.out_port.send(req3).unwrap();

        sim_runner.run();
        // now clause unit should receive 3 requests and finished the process
    }
}
