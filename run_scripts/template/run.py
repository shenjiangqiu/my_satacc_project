#!/usr/bin/python3
import checkpoint_start
import argparse
from multiprocessing import Pool
import subprocess
import sys
import os
parser = argparse.ArgumentParser(description="Run the benchmark for minisat")
parser.add_argument("--bin", dest="bin", action="store",
                    default="./minisat_release", help="the path of minisat")
parser.add_argument("--cnf-root", dest="cnf_root",
                    default="~/cnfs/", help="the root dir of cnfs")

parser.add_argument("--tail", dest="tail", default=0,
                    help="keep how many lines of the output, 0=unlimited")
parser.add_argument("--num-core", dest="num_cores", default=4,
                    help="at most use how many cpu cores to run this task")
parser.add_argument("--args", dest="args", default=None)
args = parser.parse_args()

numcores = args.num_cores
minisat_path = os.path.abspath(os.path.expanduser(args.bin))
cnf_root = os.path.abspath(os.path.expanduser(args.cnf_root))

tail = (args.tail)
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
check_points_start = checkpoint_start.start_points
print(check_points_start)

check_points = [f"{a}.checkpint" for a in cnfs]
print(check_points)


end_props_int = [i+100000 for i in check_points_start]
end_props = [str(i) for i in end_props_int]

use_tail = True
if int(tail) == 0:
    use_tail = False
else:
    use_tail = True


# cnfs=["ASG_96_len112_known_last12_2.cnf","eqspwtrc16bparrc16.cnf","Mickey_out250_known_last146_0.cnf","toughsat_23bits_0.cnf","b1904P1-8x8c6h5SAT.cnf","files.txt,MM-23-2-2-2-2-3.cnf","toughsat_25bits_1.cnf","b1904P3-8x8c11h0SAT.cnf","Grain_no_init_ver1_out200_known_last104_0.cnf","QuasiGroup-4-12_c18.cnf","toughsat_28bits_0.cnf","Bibd-sc-10-03-08_c18.cnf","Haystacks-ext-12_c18.cnf","sha1r17m145ABCD.cnf","toughsat_30bits_0.cnf","build_png.py,hcp_bij16_16.cnf","sha1r17m148ABCD_p.cnf","Trivium_no_init_out350_known_last142_1.cnf","hcp_CP20_20.cnf","sha1r17m72a.cnf","eqsparcl10bpwtrc10.cnf","hcp_CP24_24.cnf","size_4_4_4_i0418_r8.cnf","eqspdtlf14bpwtrc14.cnf","knight_20.cnf","size_5_5_5_i003_r12.cnf"]

print("going to run these cnfs:{}".format(cnfs))


def run_task(command):
    subprocess.run(command, shell=True)


commands = [f"mkdir {c};\
cd {c};\
ln -sf ../satacc_config.toml ./;ln -sf ../*.cfg ./;\
 {minisat_path} -enable-acc -load -checkpoint-name=../../new_save_checkpoints/{c}/{checkpoint} \
-end-prop={end_prop} {os.path.join(cnf_root, c)}  \
> result_{c}.txt 2> result_{c}.err"
            for c, checkpoint, end_prop in zip(cnfs, check_points, end_props)]

print("commands are {}".format(commands[0]))
print("num_cores is  {}".format(numcores))

if not os.path.exists(minisat_path):
    print("minisat not exits in {}".format(minisat_path))
    exit(-1)
for cnf in cnfs:
    checkpoint_path = f"../new_save_checkpoints/{cnf}/{cnf}.checkpint"
    if not os.path.exists(checkpoint_path):
        print(f"checkpint not exits in {checkpoint_path}")
        exit(-1)

cnf_paths = [os.path.join(cnf_root, c) for c in cnfs]
path_exits = True
for cnf_path in cnf_paths:
    if not os.path.isfile(cnf_path):
        path_exits = False
        print("cnf path not exits: {}".format(cnf_path))
if not path_exits:
    exit(-1)


with Pool(int(numcores)) as p:
    p.map(run_task, commands)
