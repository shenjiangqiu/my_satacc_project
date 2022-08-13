use std::collections::{BTreeMap, VecDeque};

use crate::{
    satacc::{icnt::IcntMsgWrapper, wating_task::WaitingTask, MemReq, SataccStatus},
    sim::{InOutPort, SimComponent},
};

use super::{AccessResult, CacheConfig, CacheId, FastCache};

/// the cache with fix time simulator
/// the hit latency is fixed
/// when miss, the latency will be `miss_latency`
pub struct CacheWithFixTime {
    pub fast_cache: FastCache,
    pub req_ports: Vec<InOutPort<IcntMsgWrapper<MemReq>>>,
    pub on_going_reqs: WaitingTask<u64>,
    pub tag_to_reqs: BTreeMap<u64, Vec<MemReq>>,
    pub hit_latency: usize,
    pub miss_latency: usize,
    pub ready_reqs: VecDeque<MemReq>,
    pub cache_id: CacheId,
}
impl CacheWithFixTime {
    pub fn new(
        config: &CacheConfig,
        req_ports: Vec<InOutPort<IcntMsgWrapper<MemReq>>>,
        hit_latency: usize,
        miss_latency: usize,
        cache_id: CacheId,
    ) -> Self {
        Self {
            fast_cache: FastCache::new(config),
            req_ports,
            on_going_reqs: WaitingTask::new(),
            tag_to_reqs: BTreeMap::new(),
            hit_latency,
            miss_latency,
            ready_reqs: VecDeque::new(),
            cache_id,
        }
    }
}

impl SimComponent for CacheWithFixTime {
    type SharedStatus = SataccStatus;
    fn update(&mut self, shared_status: &mut Self::SharedStatus, current_cycle: usize) -> bool {
        let mut busy = !self.on_going_reqs.is_empty();
        // first check if there is any request in the in_req_queues, if find, access it
        for InOutPort {
            in_port,
            out_port: _,
        } in &mut self.req_ports
        {
            if let Ok(IcntMsgWrapper {
                msg,
                mem_target_port: _,
            }) = in_port.recv()
            {
                match self.fast_cache.access(msg.addr) {
                    AccessResult::Hit(tag) => {
                        // if it's hit, if the tag is in the tag_to_reqs, means it's already in on_going_reqs, just add this req to tag_to_reqs
                        // if the tag is not in the tag_to_reqs, means it's not in on_going_reqs, add it to on_going_reqs and tag_to_reqs
                        shared_status.statistics.update_hit(&self.cache_id);
                        match self.tag_to_reqs.get_mut(&tag) {
                            Some(entry) => {
                                entry.push(msg);
                            }
                            None => {
                                // no tag in record, add it!
                                // the latency will be hit_latency
                                self.on_going_reqs
                                    .push(tag, current_cycle + self.hit_latency);
                                self.tag_to_reqs.insert(tag, vec![msg]);
                            }
                        }
                    }
                    AccessResult::Miss(tag) => {
                        shared_status.statistics.update_miss(&self.cache_id);
                        match self.tag_to_reqs.get_mut(&tag) {
                            Some(entry) => {
                                entry.push(msg);
                            }
                            None => {
                                self.on_going_reqs
                                    .push(tag, current_cycle + self.miss_latency);
                                self.tag_to_reqs.insert(tag, vec![msg]);
                            }
                        }
                    }
                }
                busy = true;
            }
        }
        // then check if there is any request in the on_going_reqs
        while let Some((leaving_cycle, tag)) = self.on_going_reqs.pop() {
            if leaving_cycle > current_cycle {
                self.on_going_reqs.push(tag, leaving_cycle);
                break;
            } else {
                self.ready_reqs
                    .extend(self.tag_to_reqs.remove(&tag).unwrap());
            }
        }
        // then push ready queue to out

        while let Some(req) = self.ready_reqs.pop_front() {
            log::debug!("send req: {:?} at cycle: {current_cycle}", req);
            let out_id = req.mem_id;
            let wathcer_id = req.watcher_pe_id;
            let req = IcntMsgWrapper {
                msg: req,
                mem_target_port: wathcer_id,
            };
            match self.req_ports[out_id].out_port.send(req) {
                Ok(_) => {
                    busy = true;
                }
                Err(e) => {
                    // cannot send to cache now
                    self.ready_reqs.push_front(e.msg);
                    break;
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
        satacc::{cache::fast_cache::CacheConfig, MemReqType},
        sim::{ChannelBuilder, SimRunner},
        test_utils,
    };

    use super::*;
    #[test]
    fn test() {
        test_utils::init();
        let channel_builder = ChannelBuilder::new();
        let (inout_base, inout_cache) = channel_builder.in_out_poat_array(1000, 2);
        let cache = CacheWithFixTime {
            fast_cache: FastCache::new(&CacheConfig {
                sets: 2,
                associativity: 2,
                block_size: 4,
                channels: 1,
            }),

            on_going_reqs: WaitingTask::new(),
            hit_latency: 14,
            miss_latency: 120,
            tag_to_reqs: BTreeMap::new(),
            ready_reqs: VecDeque::new(),
            req_ports: inout_cache,
            cache_id: CacheId::L3Cache,
        };
        let config = Config::default();
        let mut status = SataccStatus::new(config);
        for i in 0..10 {
            inout_base[0]
                .out_port
                .send(IcntMsgWrapper {
                    msg: MemReq {
                        addr: i,
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
