#ifndef rusttools_h
#define rusttools_h

/* Generated with cbindgen:0.24.3 */

/* Warning, this file is autogenerated by cbindgen. Don't modify this manually. */

#include <cstdarg>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <ostream>
#include <new>


namespace sjqrusttools {

enum class CacheType {
  Simple,
  Ramu,
};

/// the type of the dram that used to read and write data
enum class DramType {
  DDR4,
  HBM,
};

enum class IcntType {
  Mesh,
  Ring,
  Ideal,
};

enum class PresetConfigs {
  ALDRAM,
  DDR4,
  GDDR5,
  LPDDR3,
  PCM,
  STTMRAM,
  WideIO2,
  DDR3,
  DSARP,
  HBM,
  LPDDR4,
  SALP,
  TLDRAM,
  WideIO,
};

/// The type for the watcher sending to the clase
enum class WatcherToClauseType {
  /// - in this case, the watcher send clause to it's own clause unit
  /// - so the clause should use icnt to send memory request
  Streight,
  /// - in this case, the watcher send clause to dedicate clause unit
  /// - so the clause should direct to send memory request
  Icnt,
};

/// # SataccMinisatTask
/// the full task of the whole SAT solver
/// - it contains many decisions in [`SingleRoundTask`]
struct SataccMinisatTask;

struct SimulatorWapper;

struct CacheConfig {
  uint64_t sets;
  uint64_t associativity;
  uint64_t block_size;
  uint64_t channels;
};

/// the config for satacc
///
struct Config {
  WatcherToClauseType watcher_to_clause_type;
  size_t n_watchers;
  /// the number of clause unit per watcher
  size_t n_clauses;
  size_t mems;
  IcntType icnt;
  bool seq;
  bool ideal_memory;
  bool ideal_l3cache;
  size_t multi_port;
  DramType dram_config;
  IcntType watcher_to_clause_icnt;
  IcntType watcher_to_writer_icnt;
  size_t num_writer_entry;
  size_t num_writer_merge;
  bool single_watcher;
  size_t private_cache_size;
  size_t l3_cache_size;
  size_t channel_size;
  CacheType l3_cache_type;
  PresetConfigs ramu_cache_config;
  size_t hit_latency;
  size_t miss_latency;
  CacheConfig private_cache_config;
  CacheConfig l3_cache_config;
};

struct Point {
  int32_t x;
  int32_t y;
};

struct Rec {
  int32_t x;
  int32_t y;
};


extern "C" {

void add_single_watcher_clause_value_addr(SataccMinisatTask *self,
                                          uint64_t value_addr,
                                          size_t clause_id);

void add_single_watcher_task(SataccMinisatTask *self,
                             uint64_t blocker_addr,
                             uint64_t clause_addr,
                             size_t clause_id,
                             size_t processing_time,
                             size_t watcher_id);

void add_single_watcher_task_no_clause(SataccMinisatTask *self,
                                       uint64_t blocker_addr,
                                       size_t watcher_id);

void add_watcher_task(SataccMinisatTask *self,
                      uint64_t meta_data_addr,
                      uint64_t watcher_addr,
                      size_t watcher_id);

Config config_from_file(const char *path);

/// this will create a simulator task object, do not free it, it will be freed by calling `run_full_expr`
SataccMinisatTask *create_empty_task();

/// finish the simulation, this will consume the simulator
void finish_simulator(SataccMinisatTask *task, SimulatorWapper *sim);

/// get the simulator
SimulatorWapper *get_simulator();

int32_t get_x(const Point *self);

int32_t get_y(const Point *self);

/// run full simulation and will delete the task, do not use the task anymore!
void run_full_expr(SataccMinisatTask *task);

/// run a single round of simulation,
/// this will not consume any point, you can use it later
void run_single_task(SataccMinisatTask *task, SimulatorWapper *sim);

void say_hello(const Point *point, const Rec *rect);

void set_x(Point *self, int32_t x);

void set_y(Point *self, int32_t y);

void show_config(const Config *self);

void start_new_assgin(SataccMinisatTask *self);

} // extern "C"

} // namespace sjqrusttools

#endif // rusttools_h