use std::collections::BTreeMap;

use ramulator_wrapper::{PresetConfigs, RamulatorWrapper};

use crate::{
    satacc::{icnt::IcntMsgWrapper, wating_task::WaitingTask, MemReq, SataccStatus},
    sim::{InOutPort, SimComponent},
};

use super::{get_set_number_from_addr, AccessResult, CacheConfig, CacheId, FastCache};

/// # CacheWithRamulator
/// the cache with ramulator simulator
/// - when miss, it will send a request to the ramulator
/// - when the ramulator returns the result, it will resonse the request to sender with a `lit_latency`
pub struct CacheWithRamulator {
    pub fast_cache: FastCache,
    pub ramulator: RamulatorWrapper,
    pub req_ports: Vec<InOutPort<IcntMsgWrapper<MemReq>>>,
    pub on_going_reqs: WaitingTask<MemReq>,
    /// the tags the hold a vec of requests that are waiting for the ramulator
    pub on_dram_reqs: BTreeMap<u64, Vec<MemReq>>,
    pub hit_latency: usize,
    /// the mem request received from the queue, and accessed the cache, but not yet able to send to the dram.
    /// process it!
    pub temp_send_blocked_req: Option<MemReq>,
    pub cache_id: CacheId,
}

impl CacheWithRamulator {
    pub fn new(
        config: &CacheConfig,
        req_ports: Vec<InOutPort<IcntMsgWrapper<MemReq>>>,
        ramulator_preset: PresetConfigs,
        hit_latency: usize,
        cache_id: CacheId,
    ) -> Self {
        Self {
            fast_cache: FastCache::new(config),
            ramulator: RamulatorWrapper::new_with_preset(ramulator_preset, "ramu_stat.txt"),
            req_ports,
            on_going_reqs: WaitingTask::new(),
            on_dram_reqs: BTreeMap::new(),
            hit_latency,
            temp_send_blocked_req: None,
            cache_id,
        }
    }
}

impl SimComponent for CacheWithRamulator {
    type SharedStatus = SataccStatus;
    fn update(
        &mut self,
        shared_status: &mut Self::SharedStatus,
        current_cycle: usize,
    ) -> (bool, bool) {
        let mut busy = !self.on_going_reqs.is_empty() || !self.on_dram_reqs.is_empty();
        let mut updated = !self.on_dram_reqs.is_empty();
        // first check if there is any request in the in_req_queues
        if let Some(req) = self.temp_send_blocked_req.take() {
            busy = true;
            // if temp_send_blocked_req have value, first process it!
            if self.ramulator.available(req.addr, req.is_write) {
                updated = true;
                log::debug!("send blocked req to dram");
                let tag = get_set_number_from_addr(
                    req.addr,
                    self.fast_cache.get_set_bit_len(),
                    self.fast_cache.get_block_bit_len(),
                    self.fast_cache.get_channel_bit_len(),
                )
                .1;
                self.ramulator.send(tag, req.is_write);
                self.on_dram_reqs.insert(tag, vec![req]);
            } else {
                self.temp_send_blocked_req = Some(req);
            }
        } else {
            // for each inport, check if there is any request in the in_req_queues,
            // try to send it to dram, if cannot send it, put it in temp_send_blocked_req and send it next cycle
            for InOutPort {
                in_port,
                out_port: _,
            } in &mut self.req_ports
            {
                if let Ok(IcntMsgWrapper {
                    mem_target_port: _,
                    msg,
                }) = in_port.recv()
                {
                    busy = true;
                    updated = true;
                    log::debug!("recv req: {:?} at cycle: {current_cycle}", msg);
                    match self.fast_cache.access(msg.addr) {
                        AccessResult::Hit(tag) => {
                            match self.on_dram_reqs.get_mut(&tag) {
                                Some(entry) => {
                                    entry.push(msg);
                                }
                                None => {
                                    log::debug!("hit");
                                    self.on_going_reqs
                                        .push(msg, current_cycle + self.hit_latency);
                                    shared_status.statistics.update_hit(&self.cache_id);
                                }
                            };
                        }
                        AccessResult::Miss(tag) => {
                            shared_status.statistics.update_miss(&self.cache_id);
                            log::debug!("miss at cycle: {current_cycle}");

                            match self.on_dram_reqs.get_mut(&tag) {
                                Some(entry) => {
                                    entry.push(msg);
                                }
                                None => {
                                    // send it to dram
                                    let is_write = msg.is_write;
                                    if self.ramulator.available(tag, is_write) {
                                        self.on_dram_reqs.insert(tag, vec![msg]);
                                        self.ramulator.send(tag, is_write);
                                    } else {
                                        // cannot send to dram now
                                        // temporarily put it in the on_going_reqs
                                        log::debug!("cannot send to dram now, store it in temp slot : {:?} at cycle: {current_cycle}", msg);
                                        self.temp_send_blocked_req = Some(msg);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // then check if there is any request in the on_going_reqs
        while let Some((leaving_cycle, req)) = self.on_going_reqs.pop() {
            busy = true;
            updated = true;
            if leaving_cycle > current_cycle {
                // cannot send it now
                self.on_going_reqs.push(req, leaving_cycle);
                break;
            } else {
                let req_addr = req.addr;
                let watcher_id = req.watcher_pe_id;
                match self.req_ports[req.mem_id].out_port.send(IcntMsgWrapper {
                    msg: req,
                    mem_target_port: watcher_id,
                }) {
                    Ok(_) => {
                        log::debug!("send req: {:?} at cycle: {current_cycle}", req_addr);
                        busy = true;
                    }
                    Err(req) => {
                        log::debug!("cannot send req: {:?} at cycle: {current_cycle}", req_addr);
                        self.on_going_reqs.push(req.msg, leaving_cycle);
                        break;
                    }
                }
            }
        }

        // no temp blocked, check the dram reqs
        while self.ramulator.ret_available() {
            busy = true;
            updated = true;
            let tag = self.ramulator.pop();
            log::debug!("dram req: {:?} at cycle: {}", tag, current_cycle);
            match self.on_dram_reqs.remove(&tag) {
                Some(mut entrys) => {
                    while let Some(req) = entrys.pop() {
                        log::debug!(
                            "send drm req: {:?} to ongoing at cycle: {current_cycle}",
                            req
                        );
                        self.on_going_reqs
                            .push(req, current_cycle + self.hit_latency);
                    }
                }

                None => {
                    panic!("no entry for tag {}", tag);
                }
            }
        }
        self.ramulator.cycle();
        (busy, updated)
    }
}

#[cfg(test)]
mod test {
    use crate::{
        satacc::{CacheConfig, MemReqType},
        sim::{ChannelBuilder, SimRunner},
        test_utils,
    };

    use super::*;
    use ramulator_wrapper::PresetConfigs;
    #[test]
    fn test_cache_with_ramu() {
        test_utils::init();
        let channel_builder = ChannelBuilder::new();
        let (inout_base, inout_cache) = channel_builder.in_out_poat_array(1000, 2);
        let cache = CacheWithRamulator {
            fast_cache: FastCache::new(&CacheConfig {
                sets: 2,
                associativity: 2,
                block_size: 4,
                channels: 1,
                alway_hit: false,
            }),
            ramulator: RamulatorWrapper::new_with_preset(PresetConfigs::HBM, "STAT.txt"),
            on_going_reqs: WaitingTask::new(),
            on_dram_reqs: BTreeMap::new(),
            hit_latency: 14,
            temp_send_blocked_req: None,
            req_ports: inout_cache,
            cache_id: CacheId::L3Cache,
        };
        let mut status = SataccStatus::default();
        for i in 0..1000 {
            inout_base[0]
                .out_port
                .send(IcntMsgWrapper {
                    msg: MemReq {
                        addr: 0x1000 + i * 9933,
                        is_write: false,
                        mem_id: 0,
                        id: status.next_mem_id(),
                        req_type: MemReqType::WatcherReadData,
                        watcher_pe_id: 0,
                    },
                    mem_target_port: 1,
                })
                .unwrap();
        }

        let mut sim_runner = SimRunner::new(cache, status);
        sim_runner.run();
    }
}
