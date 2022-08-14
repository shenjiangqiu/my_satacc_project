# %%
import imp
import os
configs = ["1x1", "1x4", "4x4", "4x16", "16x16"]
os.mkdir("archive")


# move folder to archive
for config in configs:
    folder_name = config
    src = "./" + folder_name
    dst = "./archive/" + folder_name
    os.rename(src, dst)
    print("moved " + src + " to " + dst)

# %%
