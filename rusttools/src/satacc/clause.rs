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
    /// task_id,(num_waiting_memreq,clauseTask)
    current_waiting_reading_value_tasks: BTreeMap<usize, (usize, ClauseTask)>,
    current_waiting_reading_value_reqs: VecDeque<IcntMsgWrapper<MemReq>>,
    /// the req id to task id
    current_waiting_value_memid_to_task_id: BTreeMap<usize, usize>,
    mem_req_id_to_clause_task: BTreeMap<usize, ClauseTask>,
    total_clause_data_mem_ongoing: usize,
    total_clause_value_mem_ongoing: usize,
    current_task_id: usize,
    pipeline_clause_value_read: bool,
}
#[derive(Debug)]
enum IdleReason {
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
        pipeline_clause_value_read: bool,
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
            total_clause_data_mem_ongoing: 0,
            total_clause_value_mem_ongoing: 0,
            current_task_id: 0,
            current_waiting_reading_value_reqs: VecDeque::new(),
            current_waiting_reading_value_tasks: BTreeMap::new(),
            current_waiting_value_memid_to_task_id: BTreeMap::new(),
            pipeline_clause_value_read,
        }
    }
}

impl SimComponent for ClauseUnit {
    type SharedStatus = SataccStatus;
    /// ClauseUnit is a component that is responsible for sending data to the
    fn update(&mut self, context: &mut Self::SharedStatus, current_cycle: usize) -> (bool, bool) {
        let mut busy = self.current_processing_task.is_some();
        let mut updated = false;
        // the reason for not updated
        let mut idle_reason = IdleReason::NoTask;
        // first read the clause data
        if self.total_clause_data_mem_ongoing < 256 && self.clause_data_ready_queue.len() < 256 {
            if let Ok(task) = self.clause_task_in.recv() {
                busy = true;

                let mem_req = task.msg.get_clause_data_task(
                    context,
                    self.watcher_pe_id,
                    self.clause_pe_id,
                    self.total_watchers,
                );
                let id = mem_req.msg.id;
                match self.mem_icnt_port.out_port.send(mem_req) {
                    Ok(_) => {
                        tracing::debug!(current_cycle, "ClauseUnit Receive task! ");

                        self.mem_req_id_to_clause_task.insert(id, task.msg);
                        context.statistics.clause_statistics[self.watcher_pe_id].single_clause
                            [self.clause_pe_id]
                            .total_clause_received += 1;
                        updated = true;
                        self.total_clause_data_mem_ongoing += 1;
                    }
                    Err(_e) => {
                        // cannot send to cache now
                        // just ret the task so we don't need the target port
                        tracing::debug!(
                            current_cycle,
                            "ClauseUnit Receive task! bug cannot send to mem "
                        );
                        self.clause_task_in.ret(task);
                        idle_reason = IdleReason::SendingL3;
                    }
                }
            }
        } else {
            tracing::debug!(
                current_cycle,
                self.total_clause_data_mem_ongoing,
                clause_data_ready_queue_len = self.clause_data_ready_queue.len(),
                "ClauseUnit cannot Receive task!"
            );
        }

        // try get a task to start to read the clause value
        match self.pipeline_clause_value_read {
            true => {
                // using pipeline to read the clause value
                if self.current_waiting_reading_value_tasks.len() < 256 {
                    if let Some(task) = self.clause_data_ready_queue.pop_front() {
                        busy = true;
                        updated = true;

                        let mem_req = task.get_read_clause_value_task(
                            context,
                            self.watcher_pe_id,
                            self.clause_pe_id,
                            self.total_watchers,
                        );
                        let req_len = mem_req.len();
                        assert!(req_len != 0);
                        // the req id to task id
                        for req in mem_req.iter() {
                            self.current_waiting_value_memid_to_task_id
                                .insert(req.msg.id, self.current_task_id);
                        }
                        tracing::debug!(
                            self.current_task_id,
                            req_len,
                            req_ids = mem_req.len(),
                            "ClauseUnit add value reqs for task",
                        );
                        self.current_waiting_reading_value_reqs.extend(mem_req);

                        // task id to task
                        self.current_waiting_reading_value_tasks
                            .insert(self.current_task_id, (req_len, task));
                        self.current_task_id += 1;

                        context.statistics.clause_statistics[self.watcher_pe_id].single_clause
                            [self.clause_pe_id]
                            .total_value_read += req_len;
                    }
                } else {
                    busy = true;
                    tracing::debug!(
                        current_waiting_reading_value_tasks_len =
                            self.current_waiting_reading_value_tasks.len(),
                        "cannot send value task ",
                    );
                    // tracing::debug!(
                    //     "current_waiting_reading_value_tasks: {:?}",
                    //     self.current_waiting_reading_value_tasks
                    // );
                }
            }
            false => {
                if self.current_reading_value_task.is_none() {
                    if let Some(task) = self.clause_data_ready_queue.pop_front() {
                        busy = true;
                        updated = true;
                        let mem_req = task.get_read_clause_value_task(
                            context,
                            self.watcher_pe_id,
                            self.clause_pe_id,
                            self.total_watchers,
                        );
                        context.statistics.clause_statistics[self.watcher_pe_id].single_clause
                            [self.clause_pe_id]
                            .total_value_read +=
                            task.clause_data.as_ref().unwrap().clause_value_addr.len();
                        self.current_reading_value_task = Some(ClauseValueTracker {
                            clause_task: task,
                            waiting_to_send_reqs: mem_req.into_iter().collect(),
                            unfinished_req_id: BTreeSet::new(),
                        });
                    }
                }
            }
        };
        // process the clause value task
        if self.total_clause_value_mem_ongoing < 256 && self.clause_value_ready_queue.len() < 256 {
            match self.pipeline_clause_value_read {
                true => {
                    if let Some(req) = self.current_waiting_reading_value_reqs.pop_front() {
                        busy = true;
                        // let id = req.msg.id;
                        let req_id = req.msg.id;
                        match self.mem_icnt_port.out_port.send(req) {
                            Ok(_) => {
                                updated = true;
                                self.total_clause_value_mem_ongoing += 1;
                                tracing::debug!(
                                    req_id,
                                    current_cycle,
                                    "ClauseUnit Send private cache "
                                );
                            }
                            Err(e) => {
                                // cannot send to cache now
                                // just ret the task so we don't need the target port
                                // never ignore any value, that's a bug!
                                self.current_waiting_reading_value_reqs.push_front(e);
                                tracing::debug!(
                                    current_cycle,
                                    "ClauseUnit cannot send read value to mem "
                                );
                                idle_reason = IdleReason::SendingL1;
                            }
                        }
                    }
                }
                false => {
                    if let Some(reqs) = self.current_reading_value_task.as_mut() {
                        if let Some(req) = reqs.waiting_to_send_reqs.pop_front() {
                            busy = true;
                            let id = req.msg.id;
                            match self.mem_icnt_port.out_port.send(req) {
                                Ok(_) => {
                                    updated = true;
                                    reqs.unfinished_req_id.insert(id);
                                    self.total_clause_value_mem_ongoing += 1;
                                }
                                Err(e) => {
                                    // cannot send to cache now
                                    // just ret the task so we don't need the target port
                                    tracing::debug!(
                                        current_cycle,
                                        "ClauseUnit cannot send read value to mem "
                                    );
                                    reqs.waiting_to_send_reqs.push_front(e);
                                    idle_reason = IdleReason::SendingL1;
                                }
                            }
                        }
                    }
                }
            }
        } else {
            busy = true;
            tracing::debug!(
                "ClauseUnit cannot send read value to private cache {current_cycle},\
                total_private_mem_ongoing: {},\
                clause_value_ready_queue:len: {}",
                self.total_clause_value_mem_ongoing,
                self.clause_value_ready_queue.len()
            );
        }

        // then update current process task
        if let Some((finished_cycle, task)) = self.current_processing_task.take() {
            busy = true;
            updated = true;
            if finished_cycle >= current_cycle {
                self.current_processing_task = Some((finished_cycle, task));
            } else {
                // finished
                context.current_level_finished_tasks += 1;
                tracing::debug!(current_cycle, "ClauseUnit finished task! ");
            }
        }
        // then process the value ready task
        if self.current_processing_task.is_none() {
            if let Some(task) = self.clause_value_ready_queue.pop_front() {
                let process_time = task.get_process_time();
                busy = true;
                updated = true;
                self.current_processing_task = Some((current_cycle + process_time, task));
            }
        }
        // process memory ret
        if let Ok(mem_req) = self.mem_icnt_port.in_port.recv() {
            tracing::debug!(current_cycle, "ClauseUnit Receive mem_req! ");
            let req_id = mem_req.msg.id;
            match mem_req.msg.req_type {
                MemReqType::ClauseReadData(_) => {
                    self.total_clause_data_mem_ongoing -= 1;

                    let clause_task = self
                        .mem_req_id_to_clause_task
                        .remove(&mem_req.msg.id)
                        .unwrap();

                    self.clause_data_ready_queue.push_back(clause_task);
                }
                MemReqType::ClauseReadValue(_clause_id) => {
                    self.total_clause_value_mem_ongoing -= 1;
                    match self.pipeline_clause_value_read {
                        true => {
                            let task_id = self
                                .current_waiting_value_memid_to_task_id
                                .remove(&req_id)
                                .unwrap();
                            let (req_len, _task) = self
                                .current_waiting_reading_value_tasks
                                .get_mut(&task_id)
                                .unwrap();
                            *req_len -= 1;
                            tracing::debug!(req_len, task_id, "receive a req");
                            if *req_len == 0 {
                                let (_, clause_task) = self
                                    .current_waiting_reading_value_tasks
                                    .remove(&task_id)
                                    .unwrap();
                                self.clause_value_ready_queue.push_back(clause_task);
                            }
                        }
                        false => {
                            let current_waiting = self.current_reading_value_task.as_mut().unwrap();
                            current_waiting.unfinished_req_id.remove(&mem_req.msg.id);
                            if current_waiting.unfinished_req_id.is_empty() {
                                let current_waiting =
                                    self.current_reading_value_task.take().unwrap();
                                self.clause_value_ready_queue
                                    .push_back(current_waiting.clause_task);
                            };
                        }
                    }
                }
                _ => unreachable!(),
            }
            busy = true;
            updated = true;
        }

        // process private cache ret
        if let Ok(_mem_req) = self.private_cache_port.in_port.recv() {
            // let req_id = mem_req.msg.id;
            // tracing::debug!(
            //     req_id,
            //     current_cycle,
            //     "ClauseUnit Receive mem_req from private cache!"
            // );
            // self.total_private_mem_ongoing -= 1;
            // match mem_req.msg.req_type {
            //     _ => unreachable!(),
            // }
            // busy = true;
            // updated = true;
            unreachable!();
        }

        match updated {
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
                    idle_reason = IdleReason::WaitingL3;
                }
                // some private cache inflight
                if !self.current_waiting_reading_value_tasks.is_empty() {
                    idle_reason = IdleReason::WaitingL1;
                }
                let idle_stat = &mut context.statistics.clause_statistics[self.watcher_pe_id]
                    .single_clause[self.clause_pe_id]
                    .idle_stat;
                match idle_reason {
                    IdleReason::NoTask => idle_stat.idle_no_task += 1,
                    IdleReason::WaitingL1 => idle_stat.idle_wating_l1 += 1,
                    IdleReason::WaitingL3 => idle_stat.idle_wating_l3 += 1,
                    IdleReason::SendingL1 => idle_stat.idle_send_l1 += 1,
                    IdleReason::SendingL3 => idle_stat.idle_send_l3 += 1,
                }
            }
        }
        if busy && !updated {
            tracing::debug!(
                current_cycle,
                ?idle_reason,
                "ClauseUnit is busy! but not updated",
            );
            if context.verbose_mode {
                tracing::error!(
                    "current_waiting_reading_value_tasks: {:?}",
                    self.current_waiting_reading_value_tasks
                        .iter()
                        .map(|(k, v)| (k, v.0))
                        .collect::<Vec<_>>()
                );
                tracing::error!(
                    "current_waiting_value_req_queue mem_id: {:?}",
                    self.current_waiting_reading_value_reqs
                        .iter()
                        .map(|req| req.msg.id)
                        .collect::<Vec<_>>()
                )
            }
        }
        tracing::debug!(busy, updated);

        (busy, updated)
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
    fn test_clause_unit_no_pipe() {
        test_utils::init();
        tracing::info!("test_clause_unit_no_pipe");
        let channel_builder = ChannelBuilder::new();
        let clause_task_port = channel_builder.sim_channel(10);
        let clause_task_in = clause_task_port.1;
        let mem_icnt_port_pair = channel_builder.in_out_port(10);
        let mem_icnt_port = mem_icnt_port_pair.0;
        let private_cache_port_pair = channel_builder.in_out_port(10);
        let private_cache_port = private_cache_port_pair.0;
        let cluase_unit = ClauseUnit::new(
            clause_task_in,
            mem_icnt_port,
            private_cache_port,
            0,
            1,
            0,
            false,
        );
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
        sim_runner.run().unwrap();
        let req = mem_icnt_port_pair.1.in_port.recv().unwrap();
        tracing::debug!(?req, "should be read data");
        mem_icnt_port_pair.1.out_port.send(req).unwrap();
        sim_runner.run().unwrap();
        // now private cache should receive 3 requests
        let req1 = mem_icnt_port_pair.1.in_port.recv().unwrap();
        let req2 = mem_icnt_port_pair.1.in_port.recv().unwrap();
        let req3 = mem_icnt_port_pair.1.in_port.recv().unwrap();
        tracing::debug!("should be read value: {:?} {:?} {:?}", req1, req2, req3);
        mem_icnt_port_pair.1.out_port.send(req1).unwrap();
        mem_icnt_port_pair.1.out_port.send(req2).unwrap();
        mem_icnt_port_pair.1.out_port.send(req3).unwrap();

        sim_runner.run().unwrap();
        // now clause unit should receive 3 requests and finished the process
    }

    #[test]
    fn test_clause_unit_with_pipe() {
        test_utils::init();
        let channel_builder = ChannelBuilder::new();
        let clause_task_port = channel_builder.sim_channel(10);
        let clause_task_in = clause_task_port.1;
        let mem_icnt_port_pair = channel_builder.in_out_port(10);
        let mem_icnt_port = mem_icnt_port_pair.0;
        let private_cache_port_pair = channel_builder.in_out_port(10);
        let private_cache_port = private_cache_port_pair.0;
        let cluase_unit = ClauseUnit::new(
            clause_task_in,
            mem_icnt_port,
            private_cache_port,
            0,
            1,
            0,
            true,
        );
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
        sim_runner.run().unwrap();
        let req = mem_icnt_port_pair.1.in_port.recv().unwrap();
        tracing::debug!("should be read data: {:?}", req);
        mem_icnt_port_pair.1.out_port.send(req).unwrap();
        sim_runner.run().unwrap();
        // now private cache should receive 3 requests
        let req1 = mem_icnt_port_pair.1.in_port.recv().unwrap();
        let req2 = mem_icnt_port_pair.1.in_port.recv().unwrap();
        let req3 = mem_icnt_port_pair.1.in_port.recv().unwrap();
        tracing::debug!("should be read value: {:?} {:?} {:?}", req1, req2, req3);
        mem_icnt_port_pair.1.out_port.send(req1).unwrap();
        mem_icnt_port_pair.1.out_port.send(req2).unwrap();
        mem_icnt_port_pair.1.out_port.send(req3).unwrap();

        sim_runner.run().unwrap();
        // now clause unit should receive 3 requests and finished the process
    }
}
