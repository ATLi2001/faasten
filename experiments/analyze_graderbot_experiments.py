import json
import os
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt

# analyze the experiment results in given directory
def analyze_dir(dir_path):
    results = pd.DataFrame(columns=["function", "launched", "completed", "remaining_workflow_len", "trial"])
    for file_path in os.listdir(dir_path):
        print(file_path)
        with open(os.path.join(dir_path, file_path), "r") as f:
            for j in f.readlines():
                curr_json = json.loads(j.strip())
                # if no trial then was the warmup
                if "trial" not in curr_json["request"]["payload"]["context"]["metadata"]:
                    continue
                results.loc[len(results)] = [
                    curr_json["request"]["function"], 
                    curr_json["launched"],
                    curr_json["completed"],
                    len(curr_json["request"]["payload"]["workflow"]), 
                    curr_json["request"]["payload"]["context"]["metadata"]["trial"],
                ]

    return results

baseline = analyze_dir(os.path.join(os.curdir, "graderbot", "baseline"))
ext_sync = analyze_dir(os.path.join(os.curdir, "graderbot", "ext_sync"))

# sort by trial then remaining_workflow_len descending
baseline.sort_values(by=["trial", "remaining_workflow_len"], ascending=[True, False], inplace=True)
ext_sync.sort_values(by=["trial", "remaining_workflow_len"], ascending=[True, False], inplace=True)

# get function completition times relative to first launch time
baseline["net_completed"] = baseline["completed"].sub(baseline.groupby("trial")["launched"].transform("first"))
ext_sync["net_completed"] = ext_sync["completed"].sub(ext_sync.groupby("trial")["launched"].transform("first"))

baseline_function_means = baseline.groupby("function")["net_completed"].mean().sort_values()
ext_sync_function_means = ext_sync.groupby("function")["net_completed"].mean().sort_values()

plt.scatter(np.arange(len(baseline_function_means)), baseline_function_means / 10**6, label="baseline")
plt.scatter(np.arange(len(ext_sync_function_means)), ext_sync_function_means / 10**6, label="ext_sync")
plt.title("External Synchrony vs Baseline on Graderbot Functions")
plt.ylabel("Function Completition Time (ms)")
plt.xlabel("Function")
plt.xticks(np.arange(len(baseline_function_means)), baseline_function_means.index, fontsize=6)
plt.legend()
plt.show()