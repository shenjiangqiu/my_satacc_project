# %%
import os
import toml
import shutil
configs = ["1x1", "1x4", "4x4", "4x16", "16x16"]
for config in configs:
    if not os.path.exists(config):
        os.mkdir(config)
    num_config = config.split("x")
    watchers = int(num_config[0])
    clauses = int(num_config[1])
    clauses = int(clauses/watchers)
    shutil.copy(f"template/satacc_config.toml", config)
    shutil.copy(f"template/checkpoint_start.py", config)
    shutil.copy(f"template/run.py", config)
    config_file = toml.load(open(f"{config}/satacc_config.toml", "r"))
    config_file["n_watchers"] = watchers
    config_file["n_clauses"] = clauses
    config_file["l3_cache_type"] = "Ramu"
    config_file["ramu_cache_config"] = "HBM"
    new_config_file = toml.dumps(config_file)
    with open(f"{config}/satacc_config.toml", "w") as f:
        f.write(new_config_file)


# %%
