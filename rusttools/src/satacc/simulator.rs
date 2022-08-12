use std::collections::{BTreeMap, VecDeque};

use env_logger::Env;
use ramulator_wrapper::RamulatorWrapper;

use crate::{
    config::Config,
    satacc::{
        cache::{CacheWithFixTime, CacheWithRamulator, FastCache},
        icnt::IcntMsgWrapper,
        satacc_minisat_task::ClauseTask,
        watcher_interface::WatcherInterface,
        wating_task::WaitingTask,
        MemReq, SataccStatus,
    },
    sim::{AndSim, ChannelBuilder, Connectable, SimComponent, SimRunner, SimSender},
};

use super::{
    icnt::SimpleIcnt, satacc_minisat_task::SingleRoundTask, trail::Trail, SataccMinisatTask,
};

pub struct Simulator {
    config: Config,
}
pub struct SimulatorWapper {
    total_rounds: usize,
    task_sender: SimSender<SingleRoundTask>,
    sim_runner: SimRunner<MySataccCompType, SataccStatus>,
}
type MySataccCompType = AndSim<
    AndSim<
        AndSim<AndSim<Trail, Vec<WatcherInterface>>, SimpleIcnt<IcntMsgWrapper<MemReq>>>,
        SimpleIcnt<IcntMsgWrapper<ClauseTask>>,
    >,
    Box<dyn SimComponent<SharedStatus = SataccStatus>>,
>;
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
        env_logger::Builder::from_env(Env::default().default_filter_or("info"))
            .try_init()
            .unwrap_or_else(|_e| {
                log::error!("fail to set logger!");
            });
        let simulator = Self::new("satacc_config.toml");
        let (task_sender, comp) = simulator.build();
        let shared_status = SataccStatus::new();
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
    #[no_mangle]
    pub extern "C" fn run_single_task(task: *mut SataccMinisatTask, sim: *mut SimulatorWapper) {
        unsafe {
            let task = &mut *task;
            let sim = &mut *sim;
            sim.task_sender
                .send(task.pop_next_task().unwrap())
                .unwrap_or_else(|_e| {
                    panic!("send task error");
                });
            sim.sim_runner.run();
            sim.total_rounds += 1;
            if sim.total_rounds % 1000 == 0 {
                log::info!("total rounds: {}", sim.total_rounds);
            }
        }
    }
    /// finish the simulation, this will consume the simulator
    #[no_mangle]
    pub extern "C" fn finish_simulator(task: *mut SataccMinisatTask, sim: *mut SimulatorWapper) {
        unsafe {
            let sim = Box::from_raw(sim);
            log::info!(
                "finish simulator cycle:{}",
                sim.sim_runner.get_current_cycle()
            );

            // release the task builder
            let _task = Box::from_raw(task);
        }
    }

    /// run full simulation and will delete the task, do not use the task anymore!
    #[no_mangle]
    pub extern "C" fn run_full_expr(task: *mut SataccMinisatTask) {
        env_logger::Builder::from_env(Env::default().default_filter_or("info"))
            .try_init()
            .unwrap_or_default();
        let mut task = unsafe { Box::from_raw(task) };
        let simulator = Self::new("satacc_config.toml");
        let (task_sender, comp) = simulator.build();
        let shared_status = SataccStatus::new();
        let mut sim_runner = SimRunner::new(comp, shared_status);
        while let Some(single_round_task) = task.pop_next_task() {
            task_sender.send(single_round_task).unwrap_or_else(|_e| {
                panic!("cannot send task!");
            });
            sim_runner.run();
        }
        log::info!(
            "simulator finished! total cycles: {}",
            sim_runner.get_current_cycle(),
        );
    }
    pub fn build(&self) -> (SimSender<SingleRoundTask>, MySataccCompType) {
        let channel_builder = ChannelBuilder::new();

        // build the trail
        let trail_to_watcher_ports =
            channel_builder.sim_channel_array(self.config.channel_size, self.config.n_watchers);
        let outer_to_trail_ports = channel_builder.sim_channel(self.config.channel_size);
        let trail = Trail::new(
            trail_to_watcher_ports.0,
            outer_to_trail_ports.1,
            self.config.n_watchers,
        );

        // build the icnt from pe to cache
        let num_caches = 8;
        let (mem_icnt, cache_base_ports) = SimpleIcnt::<IcntMsgWrapper<MemReq>>::new_with_config(
            self.config.n_watchers + num_caches,
            self.config.channel_size,
            &channel_builder,
        );

        // first build the icnt from watchers to clauses

        let (clause_icnt, clause_base_port) =
            SimpleIcnt::<IcntMsgWrapper<ClauseTask>>::new_with_config(
                self.config.n_watchers,
                self.config.channel_size,
                &channel_builder,
            );

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
                    self.config.hit_latency,
                    self.config.miss_latency,
                    self.config.n_clauses,
                    watcher_pe_id,
                    self.config.n_watchers,
                )
            })
            .collect::<Vec<_>>();

        // build the caches
        let shared_l3_cache: Box<dyn SimComponent<SharedStatus = SataccStatus>> =
            match self.config.l3_cache_type {
                crate::config::CacheType::Simple => {
                    let cache = CacheWithFixTime {
                        fast_cache: FastCache::new(&self.config.l3_cache_config),
                        req_ports: cache_base_ports
                            .iter()
                            .skip(self.config.n_watchers)
                            .cloned()
                            .collect(),
                        on_going_reqs: WaitingTask::new(),
                        hit_latency: self.config.hit_latency,
                        miss_latency: self.config.miss_latency,
                        tag_to_reqs: BTreeMap::new(),
                        ready_reqs: VecDeque::new(),
                    };
                    Box::new(cache)
                }
                crate::config::CacheType::Ramu => {
                    let cache = CacheWithRamulator {
                        fast_cache: FastCache::new(&self.config.l3_cache_config),
                        ramulator: RamulatorWrapper::new_with_preset(
                            self.config.ramu_cache_config,
                            "ramulator_results.txt",
                        ),
                        req_ports: cache_base_ports
                            .iter()
                            .skip(self.config.n_watchers)
                            .cloned()
                            .collect(),
                        on_going_reqs: WaitingTask::new(),
                        on_dram_reqs: BTreeMap::new(),
                        hit_latency: self.config.hit_latency,
                        temp_send_blocked_req: None,
                    };
                    Box::new(cache)
                }
            };
        let simulator = trail
            .connect(watchers_interface)
            .connect(mem_icnt)
            .connect(clause_icnt)
            .connect(shared_l3_cache);
        (outer_to_trail_ports.0, simulator)
    }
}

#[cfg(test)]
mod test {
    use ramulator_wrapper::PresetConfigs;

    use crate::{
        config::{CacheType, Config, DramType, IcntType, WatcherToClauseType},
        satacc::{
            satacc_minisat_task::{ClauseData, ClauseTask, SingleRoundTask, WatcherTask},
            CacheConfig, SataccStatus,
        },
        sim::SimRunner,
        test_utils,
    };

    use super::Simulator;

    #[test]
    fn test_simulator() {
        test_utils::init();

        let config = Config {
            watcher_to_clause_type: WatcherToClauseType::Icnt,
            n_watchers: 4,
            n_clauses: 2,
            mems: 8,
            icnt: IcntType::Mesh,
            seq: false,
            ideal_memory: false,
            ideal_l3cache: false,
            multi_port: 1,
            dram_config: DramType::HBM,
            watcher_to_clause_icnt: IcntType::Mesh,
            watcher_to_writer_icnt: IcntType::Mesh,
            num_writer_entry: 2,
            num_writer_merge: 2,
            single_watcher: false,
            private_cache_size: 1,
            l3_cache_size: 1,
            channel_size: 16,
            l3_cache_type: CacheType::Simple,
            ramu_cache_config: PresetConfigs::HBM,
            private_cache_config: CacheConfig {
                sets: 16,
                associativity: 2,
                block_size: 64,
                channels: 1,
            },
            l3_cache_config: CacheConfig {
                sets: 16,
                associativity: 2,
                block_size: 64,
                channels: 8,
            },
            hit_latency: 10,
            miss_latency: 120,
        };
        let simulator = Simulator::new_from_config(config);
        let (task_sender, comp) = simulator.build();
        let status = SataccStatus::new();
        let mut sim_runner = SimRunner::new(comp, status);
        task_sender
            .send(SingleRoundTask {
                assignments: [WatcherTask {
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
        sim_runner.run();
    }
}
