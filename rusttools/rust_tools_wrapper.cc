#include <rust_tools_wrapper.hpp>
SatStatsWrapper::SatStatsWrapper()
    : satstats(sjqrusttools::new_satstat_pointer()) {}

SatStatsWrapper::~SatStatsWrapper() {
  sjqrusttools::delete_satstat_pointer(satstats);
}
void SatStatsWrapper::satstat_add_watcher(size_t num_clause_total,
                                          size_t num_clause_read) {
  sjqrusttools::satstat_add_watcher(satstats, num_clause_total,
                                    num_clause_read);
}
void SatStatsWrapper::end_decision(bool conflict) {
  sjqrusttools::end_decision(satstats, conflict);
}
void SatStatsWrapper::save_data() const { sjqrusttools::save_data(satstats); }
void SatStatsWrapper::show_data() const { sjqrusttools::show_data(satstats); }

// simulator wrapper
SimulatorWapper::SimulatorWapper() : sim(sjqrusttools::get_simulator()) {}
SimulatorWapper::~SimulatorWapper() { sjqrusttools::release_simulator(sim); }
void SimulatorWapper::finish_simulator() {
  sjqrusttools::finish_simulator(sim);
}
void SimulatorWapper::run_single_task(SataccMinisatTaskWrapper &task) {
  sjqrusttools::run_single_task(task.task, sim);
}

// satacc minisat task wrapper
SataccMinisatTaskWrapper::SataccMinisatTaskWrapper()
    : task(sjqrusttools::create_empty_task()) {}
SataccMinisatTaskWrapper::~SataccMinisatTaskWrapper() {
  sjqrusttools::release_task(task);
}

void SataccMinisatTaskWrapper::add_single_watcher_clause_value_addr(
    uint64_t value_addr, size_t clause_id) {
  sjqrusttools::add_single_watcher_clause_value_addr(task, value_addr,
                                                     clause_id);
}
void SataccMinisatTaskWrapper::add_single_watcher_task(uint64_t blocker_addr,
                                                       uint64_t clause_addr,
                                                       size_t clause_id,
                                                       size_t processing_time,
                                                       size_t watcher_id) {
  sjqrusttools::add_single_watcher_task(task, blocker_addr, clause_addr,
                                        clause_id, processing_time, watcher_id);
}
void SataccMinisatTaskWrapper::add_single_watcher_task_no_clause(
    uint64_t blocker_addr, size_t watcher_id) {
  sjqrusttools::add_single_watcher_task_no_clause(task, blocker_addr,
                                                  watcher_id);
}
void SataccMinisatTaskWrapper::add_watcher_task(uint64_t meta_data_addr,
                                                uint64_t watcher_addr,
                                                size_t watcher_id) {
  sjqrusttools::add_watcher_task(task, meta_data_addr, watcher_addr,
                                 watcher_id);
}
bool SataccMinisatTaskWrapper::run_full_expr() {
  return sjqrusttools::run_full_expr(task);
}
void SataccMinisatTaskWrapper::start_new_assgin() {
  sjqrusttools::start_new_assgin(task);
}
