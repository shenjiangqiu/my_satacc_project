use crate::sim::{ChannelBuilder, InOutPort, SimComponent, SimReciver, SimSender};

use super::{
    cache::{CacheId, CacheWithFixTime},
    clause::ClauseUnit,
    icnt::IcntMsgWrapper,
    satacc_minisat_task::{ClauseTask, WatcherTask},
    watcher::Watcher,
    CacheConfig, MemReq, MemReqType, SataccStatus,
};

pub struct WatcherInterface {
    /// the interface for sending and receiving mem requests
    mem_icnt_interface_receiver: SimReciver<IcntMsgWrapper<MemReq>>,
    /// the watcher process unit
    watcher: Watcher,
    /// the clause process unit
    clauses: Vec<ClauseUnit>,
    /// the interface for sending clause unit to other watcher interfaces
    task_icnt_receiver: SimReciver<IcntMsgWrapper<ClauseTask>>,

    // internal ports
    watcher_mem_sender: SimSender<IcntMsgWrapper<MemReq>>,
    clause_mem_senders: Vec<SimSender<IcntMsgWrapper<MemReq>>>,
    watcher_private_cache_sender: SimSender<IcntMsgWrapper<MemReq>>,
    clause_private_cache_senders: Vec<SimSender<IcntMsgWrapper<MemReq>>>,

    clause_task_senders: Vec<SimSender<IcntMsgWrapper<ClauseTask>>>,

    // private cache
    private_cache: CacheWithFixTime,
    private_cache_out_receiver: SimReciver<IcntMsgWrapper<MemReq>>,
    num_clauses_per_watcher: usize,
}

impl WatcherInterface {
    pub fn new(
        mem_icnt_interface: InOutPort<IcntMsgWrapper<MemReq>>,
        task_icnt: InOutPort<IcntMsgWrapper<ClauseTask>>,
        watcher_task_in: SimReciver<WatcherTask>,
        channel_builder: &ChannelBuilder,
        queue_size: usize,
        cache_config: &CacheConfig,
        hit_latency: usize,
        miss_latency: usize,
        num_clauses_per_watcher: usize,
        watcher_pe_id: usize,
        total_watchers: usize,
    ) -> Self {
        let (watcher_mem_sender, watcher_mem_receiver) = channel_builder.sim_channel(queue_size);
        let watcher_icnt_interface = InOutPort {
            in_port: watcher_mem_receiver,
            out_port: mem_icnt_interface.out_port.clone(),
        };
        let private_cache_in = channel_builder.sim_channel(queue_size);
        let private_cache_out = channel_builder.sim_channel(queue_size);
        let watcher_private_cache_in = channel_builder.sim_channel(queue_size);
        let clauses_private_cache_in = (0..num_clauses_per_watcher)
            .map(|_| channel_builder.sim_channel(queue_size))
            .fold(
                (vec![], vec![]),
                |(mut senders, mut receivers), (sender, receiver)| {
                    senders.push(sender);
                    receivers.push(receiver);
                    (senders, receivers)
                },
            );

        let watcher_unit = Watcher::new(
            watcher_task_in,
            task_icnt.out_port,
            watcher_icnt_interface,
            private_cache_in.0.clone(),
            watcher_private_cache_in.1,
            total_watchers,
            watcher_pe_id,
        );
        let (clause_task_senders, clause_mem_senders, clauses) = (0..num_clauses_per_watcher)
            .zip(clauses_private_cache_in.1)
            .enumerate()
            .map(|(clause_pe_id, (_, clause_private_cache_port))| {
                let (clause_task_sender, clause_task_receiver) =
                    channel_builder.sim_channel(queue_size);
                let (claause_mem_sender, clause_mem_receiver) =
                    channel_builder.sim_channel(queue_size);
                let clause = ClauseUnit::new(
                    clause_task_receiver,
                    InOutPort {
                        in_port: clause_mem_receiver,
                        out_port: mem_icnt_interface.out_port.clone(),
                    },
                    InOutPort {
                        in_port: clause_private_cache_port,
                        out_port: private_cache_in.0.clone(),
                    },
                    watcher_pe_id,
                    total_watchers,
                    clause_pe_id,
                );
                (clause_task_sender, claause_mem_sender, clause)
            })
            .fold(
                (vec![], vec![], vec![]),
                |(mut clause_tasks, mut clause_mems, mut clauses), (ct, cm, cl)| {
                    clause_tasks.push(ct);
                    clause_mems.push(cm);
                    clauses.push(cl);
                    (clause_tasks, clause_mems, clauses)
                },
            );
        let private_cache = CacheWithFixTime::new(
            cache_config,
            vec![InOutPort {
                in_port: private_cache_in.1,
                out_port: private_cache_out.0,
            }],
            hit_latency,
            miss_latency,
            CacheId::PrivateCache(watcher_pe_id),
        );

        Self {
            mem_icnt_interface_receiver: mem_icnt_interface.in_port,
            watcher: watcher_unit,
            clauses,
            task_icnt_receiver: task_icnt.in_port,
            watcher_mem_sender,
            clause_mem_senders,
            clause_task_senders,
            private_cache,
            private_cache_out_receiver: private_cache_out.1,
            watcher_private_cache_sender: watcher_private_cache_in.0,
            clause_private_cache_senders: clauses_private_cache_in.0,
            num_clauses_per_watcher,
        }
    }
}

impl SimComponent for WatcherInterface {
    type SharedStatus = SataccStatus;
    fn update(
        &mut self,
        shared_status: &mut Self::SharedStatus,
        current_cycle: usize,
    ) -> (bool, bool) {
        let mut busy = false;
        let mut updated = false;
        // receive the clause task
        if let Ok(clause_task) = self.task_icnt_receiver.recv() {
            busy = true;
            let id = clause_task
                .msg
                .get_inner_clause_pe_id(self.num_clauses_per_watcher);
            match self.clause_task_senders[id].send(clause_task) {
                Ok(_) => {
                    log::debug!("WatcherInterface Send task to clause:{id}! {current_cycle}");
                    updated = true;
                }
                Err(clause_task) => {
                    self.task_icnt_receiver.ret(clause_task);
                }
            }
        }
        if let Ok(mem_req) = self.mem_icnt_interface_receiver.recv() {
            busy = true;
            match mem_req.msg.req_type {
                MemReqType::ClauseReadData(clause_inner_id) => {
                    match self.clause_mem_senders[clause_inner_id].send(mem_req) {
                        Ok(_) => {
                            log::debug!(
                                "WatcherInterface Send mem req to clause:{clause_inner_id}! {current_cycle}",
                            );
                            updated = true;
                        }
                        Err(mem_req) => {
                            self.mem_icnt_interface_receiver.ret(mem_req);
                        }
                    }
                }
                MemReqType::WatcherReadMetaData | MemReqType::WatcherReadData => match self
                    .watcher_mem_sender
                    .send(mem_req)
                {
                    Ok(_) => {
                        log::debug!("WatcherInterface Send mem req to watcher! {current_cycle}");
                        updated = true;
                    }
                    Err(mem_req) => {
                        self.mem_icnt_interface_receiver.ret(mem_req);
                    }
                },
                _ => unreachable!(),
            }
        }
        // recv the private cache, it should contains clause value and watcher
        if let Ok(mem_req) = self.private_cache_out_receiver.recv() {
            busy = true;
            let msg_id = mem_req.msg.id;
            log::debug!(
                "WatcherInterface Recv mem req from private cache! id: {} cycle: {current_cycle}",
                mem_req.msg.id,
            );
            match mem_req.msg.req_type {
                MemReqType::ClauseReadValue(clause_inner_id) => {
                    match self.clause_private_cache_senders[clause_inner_id].send(mem_req) {
                        Ok(_) => {
                            log::debug!(
                                "WatcherInterface Send mem req id {msg_id} to clause:{clause_inner_id}! {current_cycle}",
                            );
                            updated = true;
                        }
                        Err(mem_req) => {
                            self.private_cache_out_receiver.ret(mem_req);
                        }
                    }
                }
                MemReqType::WatcherReadBlocker => {
                    match self.watcher_private_cache_sender.send(mem_req) {
                        Ok(_) => {
                            log::debug!("WatcherInterface Send mem req to watcher! id: {msg_id} cycle: {current_cycle}");
                            updated = true;
                        }
                        Err(mem_req) => {
                            self.private_cache_out_receiver.ret(mem_req);
                        }
                    }
                }
                _ => unreachable!(),
            }
        }
        // let (watcher_busy, watcher_updated) = self.watcher.update(shared_status, current_cycle);
        // let (clause_busy, clause_updated) = self.clauses.update(shared_status, current_cycle);
        // let (cache_busy, cache_updated) = self.private_cache.update(shared_status, current_cycle);
        let (c_busy, c_update) = (
            &mut self.watcher,
            &mut self.clauses,
            &mut self.private_cache,
        )
            .update(shared_status, current_cycle);
        let busy = busy || c_busy;
        let updated = updated || c_update;
        if busy && !updated {
            log::debug!("WatcherInterface is busy but not updated! {current_cycle}");
        }
        (busy, updated)
        // (
        //     busy || watcher_busy || clause_busy || cache_busy,
        //     updated || watcher_updated || clause_updated || cache_updated,
        // )
    }
}

#[cfg(test)]
mod test {

    use std::collections::VecDeque;

    use crate::{
        config::Config,
        satacc::{
            satacc_minisat_task::{ClauseData, ClauseTask, WatcherTask},
            watcher_interface::WatcherInterface,
            CacheConfig, SataccStatus,
        },
        sim::{ChannelBuilder, SimRunner},
        test_utils,
    };

    #[test]
    fn test_watcher_interface() {
        test_utils::init();

        let channel_builder = ChannelBuilder::new();
        let (icnt_port_base, icnt_port_in) = channel_builder.in_out_port(10);
        let (_task_port_base, task_port_in) = channel_builder.in_out_port(10);
        let (watcher_task_sender, watcher_task_receiver) = channel_builder.sim_channel(10);
        let watcher_interface = WatcherInterface::new(
            icnt_port_in,
            task_port_in,
            watcher_task_receiver,
            &channel_builder,
            10,
            &CacheConfig {
                sets: 2,
                associativity: 2,
                block_size: 4,
                channels: 1,
                alway_hit: false,
            },
            10,
            120,
            2,
            0,
            1,
        );
        let mut config = Config::default();
        config.n_clauses = 2;
        let shared_status = SataccStatus::new(config);
        let mut sim_runner = SimRunner::new(watcher_interface, shared_status);
        // send the task to watcher interface, and it will be send to watcher, the wather will send a mem req for watcher meta data
        watcher_task_sender
            .send(WatcherTask {
                meta_data_addr: 0,
                watcher_addr: 0,
                watcher_id: 0,
                single_watcher_tasks: VecDeque::new(),
            })
            .unwrap();
        sim_runner.run().unwrap();
        let req = icnt_port_base.in_port.recv().unwrap();
        log::debug!("{:?}", req);
        // send it back
        icnt_port_base.out_port.send(req).unwrap();
        sim_runner.run().unwrap();
        // the watcher will send a mem req for watcher data
        let req = icnt_port_base.in_port.recv().unwrap();
        log::debug!("{:?}", req);
        // send it back
        icnt_port_base.out_port.send(req).unwrap();
        sim_runner.run().unwrap();
        // because there are no clause in this watcher task, so no blocker request will be sent!
        assert!(icnt_port_base.in_port.recv().is_err());
    }

    #[test]
    fn test_watcher_interface_watcher_with_clause() {
        test_utils::init();
        let channel_builder = ChannelBuilder::new();
        let (icnt_port_base, icnt_port_in) = channel_builder.in_out_port(10);
        let (_task_port_base, task_port_in) = channel_builder.in_out_port(10);
        let (watcher_task_sender, watcher_task_receiver) = channel_builder.sim_channel(10);
        let watcher_interface = WatcherInterface::new(
            icnt_port_in,
            task_port_in,
            watcher_task_receiver,
            &channel_builder,
            10,
            &CacheConfig {
                sets: 2,
                associativity: 2,
                block_size: 4,
                channels: 1,
                alway_hit: false,
            },
            10,
            120,
            2,
            0,
            1,
        );
        let mut config = Config::default();
        config.n_clauses = 2;
        let shared_status = SataccStatus::new(config);
        let mut sim_runner = SimRunner::new(watcher_interface, shared_status);
        // send the task to watcher interface, and it will be send to watcher, the wather will send a mem req for watcher meta data
        let clause_task = ClauseTask {
            watcher_id: 0,
            blocker_addr: 0,
            clause_data: None,
        };
        watcher_task_sender
            .send(WatcherTask {
                meta_data_addr: 0,
                watcher_addr: 0,
                watcher_id: 0,
                single_watcher_tasks: [clause_task].into(),
            })
            .unwrap();
        sim_runner.run().unwrap();
        let req = icnt_port_base.in_port.recv().unwrap();
        log::debug!("{:?}", req);
        // send it back
        icnt_port_base.out_port.send(req).unwrap();
        sim_runner.run().unwrap();
        // the watcher will send a mem req for watcher data
        let req = icnt_port_base.in_port.recv().unwrap();
        log::debug!("{:?}", req);
        // send it back
        icnt_port_base.out_port.send(req).unwrap();
        sim_runner.run().unwrap();
        // because there are one clause task, so it will read the private cache,
        //but it will not send the clause task to other interface, because the clause is not required to read clause data
    }

    #[test]
    fn test_watcher_interface_watcher_with_clause_read_clause_datas() {
        test_utils::init();
        let channel_builder = ChannelBuilder::new();
        let (icnt_port_base, icnt_port_in) = channel_builder.in_out_port(10);
        let (task_port_base, task_port_in) = channel_builder.in_out_port(10);
        let (watcher_task_sender, watcher_task_receiver) = channel_builder.sim_channel(10);
        let watcher_interface = WatcherInterface::new(
            icnt_port_in,
            task_port_in,
            watcher_task_receiver,
            &channel_builder,
            10,
            &CacheConfig {
                sets: 2,
                associativity: 2,
                block_size: 4,
                channels: 1,
                alway_hit: false,
            },
            10,
            120,
            2,
            0,
            1,
        );
        let mut config = Config::default();
        config.n_clauses = 2;
        let shared_status = SataccStatus::new(config);
        let mut sim_runner = SimRunner::new(watcher_interface, shared_status);
        // send the task to watcher interface, and it will be send to watcher, the wather will send a mem req for watcher meta data
        let clause_task = ClauseTask {
            watcher_id: 0,
            blocker_addr: 0,
            clause_data: Some(ClauseData {
                clause_id: 1,
                clause_addr: 100,
                clause_processing_time: 100,
                clause_value_addr: vec![200, 300, 400],
                clause_value_id: vec![2, 3, 4],
            }),
        };
        watcher_task_sender
            .send(WatcherTask {
                meta_data_addr: 0,
                watcher_addr: 0,
                watcher_id: 0,
                single_watcher_tasks: [clause_task].into(),
            })
            .unwrap();
        sim_runner.run().unwrap();
        let req = icnt_port_base.in_port.recv().unwrap();
        log::debug!("{:?}", req);
        // send it back
        icnt_port_base.out_port.send(req).unwrap();
        sim_runner.run().unwrap();
        // the watcher will send a mem req for watcher data
        let req = icnt_port_base.in_port.recv().unwrap();
        log::debug!("{:?}", req);
        // send it back
        icnt_port_base.out_port.send(req).unwrap();
        sim_runner.run().unwrap();
        // because there are one clause task, so it will read the private cache,
        // and it will send the request to the clause unit.
        let req = task_port_base.in_port.recv().unwrap();
        log::debug!("{:?}", req);
        // send it back
        task_port_base.out_port.send(req).unwrap();
        sim_runner.run().unwrap();
        // now the clause unit will receive the task, and it will read the clause
        let req = icnt_port_base.in_port.recv().unwrap();
        log::debug!("{:?}", req);
        // send it back
        icnt_port_base.out_port.send(req).unwrap();
        sim_runner.run().unwrap();
        // not the clause unit will finished read clause, then access the private cache for value, then finish the task! done!
    }
}
