use std::collections::BTreeMap;

use ramulator_wrapper::RamulatorWrapper;

use crate::{
    satacc::{icnt::IcntMsgWrapper, wating_task::WaitingTask, MemReq, SataccStatus},
    sim::{InOutPort, SimComponent},
};

use super::{get_set_number_from_addr, AccessResult, FastCache};

pub struct CacheWithRamulator {
    pub fast_cache: FastCache,
    pub ramulator: RamulatorWrapper,
    pub req_ports: Vec<InOutPort<IcntMsgWrapper<MemReq>>>,
    pub on_going_reqs: WaitingTask<MemReq>,
    pub on_dram_reqs: BTreeMap<u64, Vec<MemReq>>,
    pub hit_latency: usize,
    pub temp_send_blocked_req: Option<MemReq>,
}

impl CacheWithRamulator {}

impl SimComponent for CacheWithRamulator {
    type SharedStatus = SataccStatus;
    fn update(&mut self, shared_status: &mut Self::SharedStatus, current_cycle: usize) -> bool {
        let mut busy = !self.on_going_reqs.is_empty() || !self.on_dram_reqs.is_empty();
        // first check if there is any request in the in_req_queues
        if let Some(req) = self.temp_send_blocked_req.take() {
            if self.ramulator.available(req.addr, req.is_write) {
                log::debug!("send blocked req to dram");
                let tag = get_set_number_from_addr(
                    req.addr,
                    self.fast_cache.get_set_bit_len(),
                    self.fast_cache.get_block_bit_len(),
                    self.fast_cache.get_channel_bit_len(),
                )
                .1;
                self.ramulator.send(req.addr, req.is_write);
                self.on_dram_reqs.insert(tag, vec![req]);
            } else {
                self.temp_send_blocked_req = Some(req);
            }
        } else {
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
                                    shared_status.cache_status.hits += 1;
                                }
                            };
                        }
                        AccessResult::Miss(tag) => {
                            shared_status.cache_status.misses += 1;
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
                    busy = true;
                }
            }
        }
        // then check if there is any request in the on_going_reqs
        while let Some((leaving_cycle, req)) = self.on_going_reqs.pop() {
            if leaving_cycle > current_cycle {
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
        busy
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
            }),
            ramulator: RamulatorWrapper::new_with_preset(PresetConfigs::HBM, "STAT.txt"),
            on_going_reqs: WaitingTask::new(),
            on_dram_reqs: BTreeMap::new(),
            hit_latency: 14,
            temp_send_blocked_req: None,
            req_ports: inout_cache,
        };
        let mut status = SataccStatus::new();
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
