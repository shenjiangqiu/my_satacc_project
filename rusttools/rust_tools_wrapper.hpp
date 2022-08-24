#ifndef RUST_TOOLS_WRAPPER_HPP
#define RUST_TOOLS_WRAPPER_HPP
#include <rusttools.h>
class SatStatsWrapper {
public:
  SatStatsWrapper();
  ~SatStatsWrapper();
  void end_decision(bool conflict);
  void satstat_add_watcher(size_t num_clause_total, size_t num_clause_read);
  void save_data() const;
  void show_data() const;

private:
  sjqrusttools::Satstat *const satstats;
};
class SataccMinisatTaskWrapper {
public:
  SataccMinisatTaskWrapper();

  ~SataccMinisatTaskWrapper();
  void add_single_watcher_clause_value_addr(uint64_t value_addr,
                                            size_t clause_id);
  void add_single_watcher_task(uint64_t blocker_addr, uint64_t clause_addr,
                               size_t clause_id, size_t processing_time,
                               size_t watcher_id);

  void add_single_watcher_task_no_clause(uint64_t blocker_addr,
                                         size_t watcher_id);
  void add_watcher_task(uint64_t meta_data_addr, uint64_t watcher_addr,
                        size_t watcher_id);
  bool run_full_expr();
  void start_new_assgin();

private:
  sjqrusttools::SataccMinisatTask *const task;
  friend class SimulatorWapper;
};
class SimulatorWapper {
public:
  SimulatorWapper();
  ~SimulatorWapper();
  void finish_simulator();
  void run_single_task(SataccMinisatTaskWrapper &task);

private:
  sjqrusttools::SimulatorWapper *const sim;
};

#endif