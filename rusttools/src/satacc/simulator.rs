use std::fs::File;

use serde::{Deserialize, Serialize};

use crate::{
    config::Config,
    satacc::{
        cache::{CacheWithFixTime, CacheWithRamulator},
        icnt::IcntMsgWrapper,
        satacc_minisat_task::ClauseTask,
        watcher_interface::WatcherInterface,
        MemReq, SataccStatus,
    },
    sim::{ChannelBuilder, SimComponent, SimRunner, SimSender},
};

use super::{
    cache::CacheId, icnt::SimpleIcnt, satacc_minisat_task::SingleRoundTask, trail::Trail,
    SataccMinisatTask,
};

pub struct Simulator {
    config: Config,
}
pub struct SimulatorWapper {
    total_rounds: usize,
    task_sender: SimSender<SingleRoundTask>,
    sim_runner: SimRunner<TrailAndOthers, SataccStatus>,
}
#[repr(C)]
#[derive(Debug, Deserialize, Serialize, Clone, Copy)]
pub enum RunMode {
    NoGapBtweenRounds,
    RealRoundGap,
}
pub struct TrailAndOthers {
    trail: Trail,
    others: (
        Vec<WatcherInterface>,
        SimpleIcnt<IcntMsgWrapper<MemReq>>,
        SimpleIcnt<IcntMsgWrapper<ClauseTask>>,
        Box<dyn SimComponent<SharedStatus = SataccStatus>>,
    ),
    current_running_mode: RunMode,
}

impl SimComponent for TrailAndOthers {
    type SharedStatus = SataccStatus;

    fn update(
        &mut self,
        shared_status: &mut Self::SharedStatus,
        current_cycle: usize,
    ) -> (bool, bool) {
        match self.current_running_mode {
            RunMode::NoGapBtweenRounds => {
                let trail_busy = self.trail.update(shared_status, current_cycle);
                let others_busy = self.others.update(shared_status, current_cycle);
                (trail_busy.0, trail_busy.1 || others_busy.1)
            }
            RunMode::RealRoundGap => {
                // let (trail_busy, trail_updated) = self.trail.update(shared_status, current_cycle);
                // let (others_busy, others_updated) =
                //     self.others.update(shared_status, current_cycle);
                // (trail_busy || others_busy, trail_updated || others_updated)
                (&mut self.trail, &mut self.others).update(shared_status, current_cycle)
            }
        }
    }
}

impl Simulator {
    pub fn new(config_file: &str) -> Self {
        Self {
            config: Config::from_config_file(config_file).unwrap(),
        }
    }

    pub fn new_from_config(config: Config) -> Self {
        Self { config }
    }

    /// get the simulator
    #[no_mangle]
    pub extern "C" fn get_simulator() -> *mut SimulatorWapper {
        tracing_subscriber::fmt::try_init().unwrap_or_default();
        let config = Config::from_config_file("satacc_config.toml").unwrap();

        let simulator = Self::new_from_config(config.clone());

        let (task_sender, comp) = simulator.build(config.init_running_mode);
        let shared_status = SataccStatus::new(config);
        let sim_runner = SimRunner::new(comp, shared_status);
        let wapper = SimulatorWapper {
            total_rounds: 0,
            task_sender,
            sim_runner,
        };
        Box::into_raw(Box::new(wapper))
    }

    /// run a single round of simulation,
    /// this will not consume any point, you can use it later
    /// return still ok?
    #[no_mangle]
    pub extern "C" fn run_single_task(
        task: *mut SataccMinisatTask,
        sim: *mut SimulatorWapper,
    ) -> bool {
        unsafe {
            let task = &mut *task;
            let sim = &mut *sim;
            sim.task_sender
                .send(task.pop_next_task().unwrap())
                .unwrap_or_else(|_e| {
                    panic!("send task error");
                });
            match sim.sim_runner.run() {
                Ok(_) => {
                    sim.total_rounds += 1;
                    if sim.total_rounds % 1000 == 0 {
                        tracing::info!("total rounds: {}", sim.total_rounds);
                    }
                    return true;
                }
                Err(e) => {
                    sim.sim_runner.get_shared_status_mut().verbose_mode = true;
                    // run extra 100 cycle for log infomation
                    tracing::error!(
                        "simluation start error, the remaining is the extra 100 cycles"
                    );
                    for _ in 0..100 {
                        match sim.sim_runner.run() {
                            Ok(_) => {
                                tracing::error!("cant be dead lock and resume!");
                            }
                            Err(e) => {
                                tracing::error!("simulation error: {}", e);
                            }
                        }
                    }
                    tracing::error!("simulation error: {}", e);
                    return false;
                }
            }
        }
    }
    /// finish the simulation, this will not consume any point, you can use it later
    /// return still ok?
    #[no_mangle]
    pub extern "C" fn finish_simulator(sim: *mut SimulatorWapper) -> bool {
        unsafe {
            let sim = &mut *sim;

            sim.sim_runner.get_sim_mut().current_running_mode = RunMode::RealRoundGap;
            match sim.sim_runner.run() {
                Ok(_) => {
                    tracing::info!(
                        "finish simulator cycle:{}",
                        sim.sim_runner.get_current_cycle()
                    );
                    // let (_, mut status, cycle) = sim.sim_runner.into_inner();
                    // status.statistics.total_cycle = cycle;
                    // status.save_statistics("statistics.json");
                    // serde_json::to_writer_pretty(File::create("cycle.json").unwrap(), &cycle)
                    //     .unwrap();
                    // // release the task builder
                    // let _task = Box::from_raw(task);
                    return true;
                }
                Err(_) => {
                    return false;
                }
            }
        }
    }

    /// delete the pointer
    #[no_mangle]
    pub extern "C" fn release_simulator(sim: *mut SimulatorWapper) {
        unsafe {
            let sim = Box::from_raw(sim);
            let (_, mut status, cycle) = sim.sim_runner.into_inner();
            status.statistics.total_cycle = cycle;
            status.save_statistics("statistics.json");
            serde_json::to_writer_pretty(File::create("cycle.json").unwrap(), &cycle).unwrap();
        }
    }

    /// run full simulation and will  not delete the task
    #[no_mangle]
    pub extern "C" fn run_full_expr(task: *mut SataccMinisatTask) -> bool {
        tracing_subscriber::fmt::try_init().unwrap_or_default();
        let task = unsafe { &mut *task };
        let config = Config {
            init_running_mode: RunMode::RealRoundGap,
            ..Config::from_config_file("satacc_config.toml").unwrap()
        };

        let simulator = Self::new_from_config(config.clone());
        let (task_sender, comp) = simulator.build(config.init_running_mode);
        let shared_status = SataccStatus::new(config);
        let mut sim_runner = SimRunner::new(comp, shared_status);
        while let Some(single_round_task) = task.pop_next_task() {
            task_sender.send(single_round_task).unwrap_or_else(|_e| {
                panic!("cannot send task!");
            });
            match sim_runner.run() {
                Ok(_) => {}
                Err(_) => {
                    tracing::error!("simulation error!");
                    return false;
                }
            }
        }
        tracing::info!(
            "simulator finished! total cycles: {}",
            sim_runner.get_current_cycle(),
        );
        let (_, mut status, cycle) = sim_runner.into_inner();
        status.statistics.total_cycle = cycle;
        status.save_statistics("statistics.json");
        serde_json::to_writer_pretty(File::create("cycle.json").unwrap(), &cycle).unwrap();
        return true;
    }
    /// build the simulator
    pub fn build(&self, init_runing_mode: RunMode) -> (SimSender<SingleRoundTask>, TrailAndOthers) {
        tracing::info!("build simulator with mode: {init_runing_mode:?}");
        let channel_builder = ChannelBuilder::new();

        // build the trail
        let trail_to_watcher_ports =
            channel_builder.sim_channel_array(self.config.channel_size, self.config.n_watchers);
        let outer_to_trail_ports = channel_builder.sim_channel(self.config.channel_size);
        let trail = Trail::new(
            trail_to_watcher_ports.0,
            outer_to_trail_ports.1,
            self.config.level_sync,
            self.config.n_watchers,
        );

        // build the icnt from pe to cache
        let num_caches = 8;
        let ideal_icnt = self.config.ideal_icnt;
        let (mem_icnt, cache_base_ports) = SimpleIcnt::<IcntMsgWrapper<MemReq>>::new_with_config(
            self.config.n_watchers + num_caches,
            self.config.channel_size,
            &channel_builder,
            ideal_icnt,
        );

        // first build the icnt from watchers to clauses

        let (clause_icnt, clause_base_port) =
            SimpleIcnt::<IcntMsgWrapper<ClauseTask>>::new_with_config(
                self.config.n_watchers,
                self.config.channel_size,
                &channel_builder,
                ideal_icnt,
            );
        let private_cache_miss_latency = if self.config.value_miss_hit_l3 {
            self.config.l3_hit_latency
        } else {
            self.config.miss_latency
        };
        // build watchers and clauses
        let watchers_interface = clause_base_port
            .into_iter()
            .zip(trail_to_watcher_ports.1)
            .zip(cache_base_ports.iter().take(self.config.n_watchers))
            .enumerate()
            .map(|(watcher_pe_id, ((icnt_port, trail_port), cache_port))| {
                WatcherInterface::new(
                    cache_port.clone(),
                    icnt_port,
                    trail_port,
                    &channel_builder,
                    self.config.channel_size,
                    &self.config.private_cache_config,
                    self.config.l1_hit_latency,
                    private_cache_miss_latency, // the miss latency for l1 is the hit latency for l3
                    self.config.n_clauses,
                    watcher_pe_id,
                    self.config.n_watchers,
                    self.config.pipeline_clause_value_read,
                )
            })
            .collect::<Vec<_>>();

        // build the caches
        let shared_l3_cache: Box<dyn SimComponent<SharedStatus = SataccStatus>> =
            match self.config.l3_cache_type {
                crate::config::CacheType::Simple => {
                    let cache = CacheWithFixTime::new(
                        &self.config.l3_cache_config,
                        cache_base_ports
                            .iter()
                            .skip(self.config.n_watchers)
                            .cloned()
                            .collect(),
                        self.config.l3_hit_latency,
                        self.config.miss_latency,
                        CacheId::L3Cache,
                    );

                    Box::new(cache)
                }
                crate::config::CacheType::Ramu => {
                    let cache = CacheWithRamulator::new(
                        &self.config.l3_cache_config,
                        cache_base_ports
                            .iter()
                            .skip(self.config.n_watchers)
                            .cloned()
                            .collect(),
                        self.config.ramu_cache_config,
                        self.config.l3_hit_latency,
                        CacheId::L3Cache,
                    );

                    Box::new(cache)
                }
            };
        let simulator = TrailAndOthers {
            trail,
            others: (watchers_interface, mem_icnt, clause_icnt, shared_l3_cache),
            current_running_mode: init_runing_mode,
        };

        (outer_to_trail_ports.0, simulator)
    }
}

#[cfg(test)]
mod test {

    use crate::{
        config::Config,
        satacc::{
            satacc_minisat_task::{ClauseData, ClauseTask, SingleRoundTask, WatcherTask},
            SataccMinisatTask, SataccStatus,
        },
        sim::SimRunner,
        test_utils,
    };

    use super::Simulator;

    #[test]
    fn test_simulator() {
        test_utils::init();

        let config = Config::default();
        let simulator = Simulator::new_from_config(config.clone());
        let (task_sender, comp) = simulator.build(config.init_running_mode);
        let status = SataccStatus::new(config);
        let mut sim_runner = SimRunner::new(comp, status);
        task_sender
            .send(SingleRoundTask {
                assignments: [WatcherTask {
                    level: 0,
                    meta_data_addr: 0,
                    watcher_addr: 100,
                    watcher_id: 1,
                    single_watcher_tasks: [ClauseTask {
                        watcher_id: 1,
                        blocker_addr: 1000,
                        clause_data: Some(ClauseData {
                            clause_id: 1,
                            clause_addr: 2000,
                            clause_processing_time: 200,
                            clause_value_addr: [3000, 4000, 5000].into(),
                            clause_value_id: [1, 2, 3].into(),
                        }),
                    }]
                    .into(),
                }]
                .into(),
            })
            .unwrap_or_else(|_| {});
        sim_runner.run().unwrap();
    }
    #[test]
    fn test_c_interface_single_round() {
        test_utils::init();
        let simulator_wrapper = Simulator::get_simulator();
        let task_builder = SataccMinisatTask::create_empty_task();
        unsafe {
            let unowned_task_builder = &mut *task_builder;
            unowned_task_builder.start_new_assgin();
            unowned_task_builder.add_watcher_task(0, 0, 1, 0);
            unowned_task_builder.add_single_watcher_task(0, 0, 0, 1, 0);
            unowned_task_builder.add_single_watcher_clause_value_addr(0, 0);
        }
        Simulator::run_single_task(task_builder, simulator_wrapper);
        Simulator::finish_simulator(simulator_wrapper);
        Simulator::release_simulator(simulator_wrapper);
        SataccMinisatTask::release_task(task_builder);
    }

    #[test]
    fn test_c_interface_all() {
        let task_builder = SataccMinisatTask::create_empty_task();
        unsafe {
            let unowned_task_builder = &mut *task_builder;
            unowned_task_builder.start_new_assgin();
            unowned_task_builder.add_watcher_task(0, 0, 1, 0);
            unowned_task_builder.add_single_watcher_task(0, 0, 0, 1, 0);
            unowned_task_builder.add_single_watcher_clause_value_addr(0, 0);
        }
        Simulator::run_full_expr(task_builder);
        SataccMinisatTask::release_task(task_builder);
    }

    #[test]
    fn test_simulator_level_sync() {
        test_utils::init();

        let config = Config {
            level_sync: true,
            ..Default::default()
        };

        let simulator = Simulator::new_from_config(config.clone());
        let (task_sender, comp) = simulator.build(config.init_running_mode);
        let status = SataccStatus::new(config);
        let mut sim_runner = SimRunner::new(comp, status);
        task_sender
            .send(SingleRoundTask {
                assignments: [
                    WatcherTask {
                        level: 1,
                        meta_data_addr: 0,
                        watcher_addr: 100,
                        watcher_id: 1,
                        single_watcher_tasks: [
                            ClauseTask {
                                watcher_id: 1,
                                blocker_addr: 1000,
                                clause_data: Some(ClauseData {
                                    clause_id: 1,
                                    clause_addr: 2000,
                                    clause_processing_time: 200,
                                    clause_value_addr: [3000, 4000, 5000].into(),
                                    clause_value_id: [1, 2, 3].into(),
                                }),
                            },
                            ClauseTask {
                                watcher_id: 1,
                                blocker_addr: 1000,
                                clause_data: None,
                            },
                        ]
                        .into(),
                    },
                    WatcherTask {
                        level: 1,
                        meta_data_addr: 0,
                        watcher_addr: 100,
                        watcher_id: 1,
                        single_watcher_tasks: [ClauseTask {
                            watcher_id: 1,
                            blocker_addr: 1000,
                            clause_data: Some(ClauseData {
                                clause_id: 1,
                                clause_addr: 2000,
                                clause_processing_time: 200,
                                clause_value_addr: [3000, 4000, 5000].into(),
                                clause_value_id: [1, 2, 3].into(),
                            }),
                        }]
                        .into(),
                    },
                    WatcherTask {
                        level: 2,
                        meta_data_addr: 0,
                        watcher_addr: 100,
                        watcher_id: 1,
                        single_watcher_tasks: [ClauseTask {
                            watcher_id: 1,
                            blocker_addr: 1000,
                            clause_data: Some(ClauseData {
                                clause_id: 1,
                                clause_addr: 2000,
                                clause_processing_time: 200,
                                clause_value_addr: [3000, 4000, 5000].into(),
                                clause_value_id: [1, 2, 3].into(),
                            }),
                        }]
                        .into(),
                    },
                ]
                .into(),
            })
            .unwrap_or_else(|_| {});
        sim_runner.run().unwrap();
    }
}
