use std::collections::BTreeMap;

use ndarray::array;
use ndarray_stats::histogram::{Bins, Edges, Grid, Histogram};
use serde::Serialize;

use crate::init_tracing;

/// the sat runtime statistics
pub struct Satstat {
    current_decision_cf: usize,
    current_watchers_cf: usize,
    current_clauses_cf: usize,
    current_clause_with_data_cf: usize,

    current_watcher_dc: usize,
    current_clause_dc: usize,
    current_clause_with_data_dc: usize,

    /// per watcher clauses
    clauses_per_watcher: Histogram<usize>,
    /// per watcher clause read
    clause_read_per_watcher: Histogram<usize>,
    /// per decision watchers
    watchers_per_decision: Histogram<usize>,
    /// per decision clause read
    clause_read_per_decision: Histogram<usize>,
    // per decision clauses
    clauses_per_decision: Histogram<usize>,
    // per conflict decisions
    decisions_per_conflict: Histogram<usize>,
    // per conflict watchers
    watchers_per_conflict: Histogram<usize>,
    // per conflict clauses
    clauses_per_conflict: Histogram<usize>,
    // per conflict clause with data
    clauses_per_conflict_with_data: Histogram<usize>,

    total_conflicts: usize,
    total_decisions: usize,
    total_clauses: usize,
    total_watchers: usize,

    total_clauses_with_data: usize,
}
impl Satstat {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for Satstat {
    fn default() -> Self {
        let edges = Edges::from(vec![
            0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100, 200, 300, 400, 500, 600, 700, 800, 900,
            1000, 2000, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10000, 20000, 30000, 40000,
            50000, 60000, 70000, 80000, 90000, 100000, 200000, 300000, 400000, 500000, 600000,
            700000, 800000, 900000, 1000000, 2000000, 3000000, 4000000, 5000000, 6000000, 7000000,
            8000000, 9000000, 10000000, 20000000, 30000000, 40000000, 50000000, 60000000, 70000000,
            80000000, 90000000, 100000000, 200000000, 300000000, 400000000, 500000000, 600000000,
            700000000, 800000000, 900000000, 1000000000,
        ]);
        let bins = Bins::new(edges);
        let grid = Grid::from(vec![bins]);
        Self {
            // per_decision_level_watcher_histogram: histogram,
            total_conflicts: 0,
            total_decisions: 0,
            total_clauses: 0,
            total_watchers: 0,
            total_clauses_with_data: 0,
            current_watchers_cf: 0,
            current_clauses_cf: 0,
            current_clause_with_data_cf: 0,
            current_decision_cf: 0,
            current_clause_dc: 0,
            current_clause_with_data_dc: 0,
            current_watcher_dc: 0,
            watchers_per_decision: Histogram::new(grid.clone()),
            clauses_per_watcher: Histogram::new(grid.clone()),
            clause_read_per_watcher: Histogram::new(grid.clone()),
            clause_read_per_decision: Histogram::new(grid.clone()),
            clauses_per_decision: Histogram::new(grid.clone()),
            decisions_per_conflict: Histogram::new(grid.clone()),
            watchers_per_conflict: Histogram::new(grid.clone()),
            clauses_per_conflict: Histogram::new(grid.clone()),
            clauses_per_conflict_with_data: Histogram::new(grid.clone()),
        }
    }
}
#[derive(Serialize)]
struct HistogramData {
    bins: Vec<usize>,
    counts: Vec<usize>,
}

#[derive(Serialize)]
struct FinalResult {
    total_conflicts: usize,
    total_decisions: usize,
    total_clauses: usize,
    total_watchers: usize,
    total_clauses_with_data: usize,
    histograms: BTreeMap<String, HistogramData>,
}
impl Satstat {
    #[no_mangle]
    pub extern "C" fn new_satstat_pointer() -> *mut Satstat {
        init_tracing();
        tracing::info!("new_satstat_pointer");
        Box::into_raw(Box::new(Satstat::default()))
    }
    #[no_mangle]
    pub extern "C" fn delete_satstat_pointer(satstat: *mut Satstat) {
        unsafe {
            Box::from_raw(satstat);
        }
    }
    #[no_mangle]
    pub extern "C" fn show_data(&self) {
        tracing::info!("total_conflicts: {}", self.total_conflicts);
        tracing::info!("total_decisions: {}", self.total_decisions);
        tracing::info!("total_clauses: {}", self.total_clauses);
        tracing::info!("total_watchers: {}", self.total_watchers);
        tracing::info!("total_clauses_with_data: {}", self.total_clauses_with_data);
        let grid = self.decisions_per_conflict.grid();
        let decisions_per_conflict_count = self.decisions_per_conflict.counts();
        tracing::info!(?grid, ?decisions_per_conflict_count);

        let grid = self.watchers_per_conflict.grid();
        let watchers_per_conflict_count = self.watchers_per_conflict.counts();
        tracing::info!(?grid, ?watchers_per_conflict_count);
        let grid = self.clauses_per_conflict.grid();
        let clauses_per_conflict_count = self.clauses_per_conflict.counts();
        tracing::info!(?grid, ?clauses_per_conflict_count);
        let grid = self.clauses_per_conflict_with_data.grid();
        let clauses_per_conflict_with_data_count = self.clauses_per_conflict_with_data.counts();
        tracing::info!(?grid, ?clauses_per_conflict_with_data_count);
        let grid = self.watchers_per_decision.grid();
        let watchers_per_decision_count = self.watchers_per_decision.counts();
        tracing::info!(?grid, ?watchers_per_decision_count);
        let grid = self.clauses_per_decision.grid();
        let clauses_per_decision_count = self.clauses_per_decision.counts();
        tracing::info!(?grid, ?clauses_per_decision_count);
        let grid = self.clause_read_per_decision.grid();
        let clause_read_per_decision_count = self.clause_read_per_decision.counts();
        tracing::info!(?grid, ?clause_read_per_decision_count);
        let grid = self.clause_read_per_watcher.grid();
        let clause_read_per_watcher_count = self.clause_read_per_watcher.counts();
        tracing::info!(?grid, ?clause_read_per_watcher_count);
        let grid = self.clauses_per_watcher.grid();
        let clauses_per_watcher_count = self.clauses_per_watcher.counts();
        tracing::info!(?grid, ?clauses_per_watcher_count);
    }
    /// called every time a propagation occurs, that is, one watcher
    #[no_mangle]
    pub extern "C" fn satstat_add_watcher(
        &mut self,
        num_clause_total: usize,
        num_clause_read: usize,
    ) {
        self.current_clause_dc += num_clause_total;
        self.current_clause_with_data_dc += num_clause_read;
        self.current_watcher_dc += 1;

        self.current_watchers_cf += 1;
        self.current_clauses_cf += num_clause_total;
        self.current_clause_with_data_cf += num_clause_read;

        self.total_clauses += num_clause_total;
        self.total_clauses_with_data += num_clause_read;
        self.total_watchers += 1;

        self.clauses_per_watcher
            .add_observation(&array![num_clause_total])
            .unwrap();
        self.clause_read_per_watcher
            .add_observation(&array![num_clause_read])
            .unwrap();
    }
    #[no_mangle]
    pub extern "C" fn end_decision(&mut self, conflict: bool) {
        self.total_decisions += 1;
        self.current_decision_cf += 1;

        self.watchers_per_decision
            .add_observation(&array![self.current_watcher_dc])
            .unwrap();
        self.current_watcher_dc = 0;
        self.clause_read_per_decision
            .add_observation(&array![self.current_clause_with_data_dc])
            .unwrap();
        self.current_clause_with_data_dc = 0;
        self.clauses_per_decision
            .add_observation(&array![self.current_clause_dc])
            .unwrap();
        self.current_clause_dc = 0;

        if conflict {
            self.total_conflicts += 1;
            self.decisions_per_conflict
                .add_observation(&array![self.current_decision_cf])
                .unwrap();
            self.current_decision_cf = 0;
            self.watchers_per_conflict
                .add_observation(&array![self.current_watchers_cf])
                .unwrap();
            self.current_watchers_cf = 0;
            self.clauses_per_conflict
                .add_observation(&array![self.current_clauses_cf])
                .unwrap();
            self.current_clauses_cf = 0;
            self.clauses_per_conflict_with_data
                .add_observation(&array![self.current_clause_with_data_cf])
                .unwrap();
            self.current_clause_with_data_cf = 0;
        }
    }
}

#[cfg(test)]
mod test {
    use ndarray::array;
    use ndarray_stats::histogram::{Bins, Edges, Grid, Histogram};

    use crate::test_utils::init;

    #[test]
    fn test_histogram() {
        let edges = Edges::from(vec![0, 10, 20, 30]);
        let bins = Bins::new(edges);
        let square_grid = Grid::from(vec![bins.clone(), bins.clone()]);
        let mut histogram = Histogram::new(square_grid);

        let observation = array![3, 3];

        histogram.add_observation(&observation).unwrap();

        let histogram_matrix = histogram.counts();
        println!("{:?}", histogram_matrix);
        println!("{:?}", histogram.grid());
    }
    #[test]
    fn test_interface() {
        init();
        let mut satstat = super::Satstat::default();
        satstat.satstat_add_watcher(10, 4);
        satstat.satstat_add_watcher(20, 3);
        satstat.end_decision(false);
        satstat.satstat_add_watcher(20, 3);
        satstat.satstat_add_watcher(20, 3);
        satstat.end_decision(true);
        satstat.show_data();
    }
}
