import json
import os
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt

# analyze the experiment results in given directory
def analyze_dir(dir_path):
    results = pd.DataFrame(columns=["interop_compute_ms", "runtime (ns)"])
    for file_path in os.listdir(dir_path):
        print(file_path)
        with open(os.path.join(dir_path, file_path), "r") as f:
            curr_experiment = json.load(f)
            # split[-1] give tms.json, want to get t
            interop_compute_ms = file_path.split("_")[-1][:-7]
            results.loc[len(results)] = [int(interop_compute_ms), curr_experiment["completed"] - curr_experiment["launched"]]

    return results

baseline = analyze_dir(os.path.join(os.curdir, "synthetic", "single", "baseline"))
ext_sync = analyze_dir(os.path.join(os.curdir, "synthetic", "single", "ext_sync"))

# sort by interop_compute_ms
baseline.sort_values(by="interop_compute_ms", inplace=True)
ext_sync.sort_values(by="interop_compute_ms", inplace=True)

print(baseline)
print(ext_sync)

x = baseline["interop_compute_ms"].to_numpy()
ext_sync_y = ext_sync["runtime (ns)"].to_numpy()
baseline_y = baseline["runtime (ns)"].to_numpy()
pct_improve = (baseline_y - ext_sync_y) / baseline_y

print(pct_improve)
plt.scatter(x, pct_improve)
plt.title("External Synchrony Improvement vs Interop Delay")
plt.ylabel("Percent Improvment")
plt.xlabel("Inter Operation Delay (ms)")
plt.show()