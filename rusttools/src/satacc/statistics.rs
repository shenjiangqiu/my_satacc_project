use serde::{Deserialize, Serialize};

use crate::config::Config;

use super::{cache::CacheId, satacc_minisat_task::SingleRoundTask};
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct CacheStatistics {
    pub cache_hits: usize,
    pub cache_misses: usize,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct WatcherIdleStat {
    pub idle_no_task: usize,
    pub idle_wating_l1: usize,
    pub idle_wating_l3: usize,
    pub idle_send_l1: usize,
    pub idle_send_l3: usize,
    pub idle_send_clause: usize,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct WatcherStatistics {
    pub total_assignments: usize,
    pub total_watchers: usize,
    pub total_clauses_sent: usize,
    pub idle_cycle: usize,
    pub busy_cycle: usize,
    pub idle_stat: WatcherIdleStat,
}
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ClauseIdleStat {
    pub idle_no_task: usize,
    pub idle_wating_l1: usize,
    pub idle_wating_l3: usize,
    pub idle_send_l1: usize,
    pub idle_send_l3: usize,
}
#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct SingleClauseStatistics {
    pub total_clause_received: usize,
    pub total_value_read: usize,
    pub idle_cycle: usize,
    pub busy_cycle: usize,
    pub idle_stat: ClauseIdleStat,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ClauseStatistics {
    pub single_clause: Vec<SingleClauseStatistics>,
}
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AverageStat {
    pub count: usize,
    pub total: usize,
}
impl AverageStat {
    pub fn add(&mut self, value: usize) {
        self.count += 1;
        self.total += value;
    }
    pub fn get_average(&self) -> f64 {
        self.total as f64 / self.count as f64
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct IcntStat {
    pub total_messages: usize,
    pub average_latency: AverageStat,
    pub idle_cycle: usize,
    pub busy_cycle: usize,
}
#[derive(Debug, Serialize, Deserialize)]
pub struct Statistics {
    pub total_cycle: usize,
    pub average_assignments: AverageStat,
    pub average_watchers: AverageStat,
    pub average_clauses: AverageStat,
    pub watcher_statistics: Vec<WatcherStatistics>,
    pub clause_statistics: Vec<ClauseStatistics>,
    pub private_cache_statistics: Vec<CacheStatistics>,
    pub icnt_statistics: IcntStat,
    pub l3_cache_statistics: CacheStatistics,
    pub config: Config,
}
impl Default for Statistics {
    fn default() -> Self {
        let config = Config::default();
        Statistics::new(config)
    }
}
impl Statistics {
    pub fn new(config: Config) -> Self {
        let n_watchers = config.n_watchers;
        let n_clauses = config.n_clauses;

        Self {
            config,
            watcher_statistics: vec![WatcherStatistics::default(); n_watchers],
            clause_statistics: vec![
                ClauseStatistics {
                    single_clause: vec![SingleClauseStatistics::default(); n_clauses],
                };
                n_watchers
            ],
            private_cache_statistics: vec![CacheStatistics::default(); n_watchers],
            l3_cache_statistics: Default::default(),
            total_cycle: 0,
            average_assignments: Default::default(),
            average_watchers: Default::default(),
            average_clauses: Default::default(),
            icnt_statistics: IcntStat::default(),
        }
    }
    pub fn update_hit(&mut self, cache_id: &CacheId) {
        match cache_id {
            CacheId::PrivateCache(cache_id) => {
                self.private_cache_statistics[*cache_id].cache_hits += 1;
            }
            CacheId::L3Cache => {
                self.l3_cache_statistics.cache_hits += 1;
            }
        }
    }
    pub fn update_miss(&mut self, cache_id: &CacheId) {
        match cache_id {
            CacheId::PrivateCache(cache_id) => {
                self.private_cache_statistics[*cache_id].cache_misses += 1;
            }
            CacheId::L3Cache => {
                self.l3_cache_statistics.cache_misses += 1;
            }
        }
    }

    /// update each round's statistics
    pub fn update_single_round_task(&mut self, single_round_task: &SingleRoundTask) {
        let single_round_stats = single_round_task.get_statistics();
        self.average_assignments
            .add(single_round_stats.total_assignments);
        self.average_watchers.add(single_round_stats.total_watchers);
        self.average_clauses.add(single_round_stats.total_clauses);
    }
}

#[cfg(test)]
mod test {
    use std::fs::File;

    use super::*;
    #[test]
    fn test_save_statistics() {
        let stat = Statistics::new(Config::default());
        let path = "stat.json";
        serde_json::to_writer_pretty(File::create(path).unwrap(), &stat).unwrap();
    }
}
