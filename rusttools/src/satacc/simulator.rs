use std::collections::{BTreeMap, VecDeque};

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
    sim::{ChannelBuilder, Connectable, SimComponent, SimRunner, SimSender},
};

use super::{
    icnt::SimpleIcnt, satacc_minisat_task::SingleRoundTask, trail::Trail, SataccMinisatTask,
};

pub struct Simulator {
    config: Config,
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
    #[no_mangle]
    pub extern "C" fn run(task: *mut SataccMinisatTask) {
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
    pub fn build(
        &self,
    ) -> (
        SimSender<SingleRoundTask>,
        impl SimComponent<SharedStatus = SataccStatus>,
    ) {
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
                    self.config.n_clauses,
                    watcher_pe_id,
                    self.config.n_watchers,
                )
            })
            .collect::<Vec<_>>();

        // build the caches
        let shared_l3_cache: Box<dyn SimComponent<SharedStatus = SataccStatus>> =
            match self.config.cache_type {
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
