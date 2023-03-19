import json
import os
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt

# analyze the experiment results in given directory
def analyze_dir(dir_path):
    results = pd.DataFrame(columns=["function", "launched", "completed", "remaining_workflow_len"])
    for file_path in os.listdir(dir_path):
        print(file_path)
        with open(os.path.join(dir_path, file_path), "r") as f:
            curr_json = json.load(f)
            results.loc[len(results)] = [
                curr_json["request"]["function"], 
                curr_json["launched"],
                curr_json["completed"],
                len(curr_json["request"]["payload"]["workflow"]), 
            ]

    return results

baseline = analyze_dir(os.path.join(os.curdir, "graderbot", "baseline"))
ext_sync = analyze_dir(os.path.join(os.curdir, "graderbot", "ext_sync"))

# sort by remaining_workflow_len descending
baseline.sort_values(by="remaining_workflow_len", ascending=False, inplace=True)
ext_sync.sort_values(by="remaining_workflow_len", ascending=False, inplace=True)

print(baseline)
print(ext_sync)

# get function completition times relative to first launch time
baseline_y = baseline["completed"].to_numpy() - baseline["launched"].to_numpy()[0]
ext_sync_y = ext_sync["completed"].to_numpy() - ext_sync["launched"].to_numpy()[0]

plt.scatter(np.arange(len(baseline_y)), baseline_y / 10**6, label="baseline")
plt.scatter(np.arange(len(ext_sync_y)), ext_sync_y / 10**6, label="ext_sync")
plt.title("External Synchrony vs Baseline on Graderbot Functions")
plt.ylabel("Function Completition Time (ms)")
plt.xlabel("Function")
plt.xticks(np.arange(len(baseline_y)), baseline["function"], fontsize=6)
plt.legend()
plt.show()