# %%
import json
import re
tasks = ["1x1", "1x4", "4x4", "4x16", "16x16"]
cnfs = [
    "b1904P1-8x8c6h5SAT.cnf",
    "b1904P3-8x8c11h0SAT.cnf",
    "eqsparcl10bpwtrc10.cnf",
    "eqspdtlf14bpwtrc14.cnf",
    "eqspwtrc16bparrc16.cnf",
    "Grain_no_init_ver1_out200_known_last104_0.cnf",
    "Haystacks-ext-12_c18.cnf",
    "hcp_bij16_16.cnf",
    "hcp_CP20_20.cnf",
    "hcp_CP24_24.cnf",
    "knight_20.cnf",
    "Mickey_out250_known_last146_0.cnf",
    "MM-23-2-2-2-2-3.cnf",
    "QuasiGroup-4-12_c18.cnf",
    "sha1r17m145ABCD.cnf",
    "sha1r17m72a.cnf",
    "size_4_4_4_i0418_r8.cnf",
    "size_5_5_5_i003_r12.cnf",
    "toughsat_28bits_0.cnf",
    "toughsat_30bits_0.cnf",
    "Trivium_no_init_out350_known_last142_1.cnf"
]

decoder = json.decoder.JSONDecoder()

cycles = dict()
average_assignments = dict()
average_watchers = dict()
average_clauses = dict()
l3_cache_statistics = dict()
l1_cache_miss_rate = dict()
watcher_idle_rate = dict()
clause_idle_rate = dict()
# %%


def average_of_list(data, name):
    target_data = [v[name] for v in data]
    return sum(target_data)/len(target_data)


for config in tasks:
    cycles[config] = dict()
    average_assignments[config] = dict()
    average_watchers[config] = dict()
    average_clauses[config] = dict()
    l3_cache_statistics[config] = dict()

    l1_cache_miss_rate[config] = dict()
    watcher_idle_rate[config] = dict()
    clause_idle_rate[config] = dict()
    for cnf in cnfs:
        file = f"{config}/{cnf}/statistics.json"
        with open(file) as f:
            data = decoder.decode(f.read())
            cycles[config][cnf] = data["total_cycle"]
            average_assignments[config][cnf] = data["average_assignments"]["total"] / \
                data["average_assignments"]["count"]
            average_watchers[config][cnf] = data["average_watchers"]["total"] / \
                data["average_watchers"]["count"]
            average_clauses[config][cnf] = data["average_clauses"]["total"] / \
                data["average_clauses"]["count"]
            cache_data = data["l3_cache_statistics"]
            hits = cache_data["cache_hits"]
            misses = cache_data["cache_misses"]
            miss_rate = misses/(hits+misses)
            l3_cache_statistics[config][cnf] = miss_rate

            l1_cache_data_array = data["private_cache_statistics"]
            cache_hits = average_of_list(l1_cache_data_array, "cache_hits")
            cache_misses = average_of_list(l1_cache_data_array, "cache_misses")
            cache_miss_rate = cache_misses/(cache_hits+cache_misses)
            l1_cache_miss_rate[config][cnf] = cache_miss_rate

            watcher_idle_data_array = data["watcher_statistics"]
            idle_cycls = average_of_list(
                watcher_idle_data_array, "idle_cycle")
            busy_cycle = average_of_list(
                watcher_idle_data_array, "busy_cycle")
            idle_rate = idle_cycls/(idle_cycls+busy_cycle)
            watcher_idle_rate[config][cnf] = idle_rate

            clause_data = data["clause_statistics"]
            total_idle = 0
            total_busy = 0
            for single_clause in clause_data:
                for clause in single_clause["single_clause"]:
                    total_idle += clause["idle_cycle"]
                    total_busy += clause["busy_cycle"]
            idle_rate = total_idle/(total_idle+total_busy)
            clause_idle_rate[config][cnf] = idle_rate

# %%


def print_data(data):
    print("configs: ", end=" ")
    for config in tasks:
        print(config, end=" ")
    print()
    for cnf in cnfs:
        print(cnf, end=" ")
        for config in tasks:
            print(data[config][cnf], end=" ")
        print()


# %%
for data_to_print in [cycles, average_assignments, average_watchers,
                      average_clauses, l3_cache_statistics, l1_cache_miss_rate,
                      watcher_idle_rate, clause_idle_rate]:
    print_data(data_to_print)
    print()

# %%
