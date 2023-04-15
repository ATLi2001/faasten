import json
import os
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt

plt.rcParams.update({"text.usetex": True})
plt.style.use("fivethirtyeight")

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

baseline = analyze_dir(os.path.join(os.curdir, "graderbot_tikv", "baseline"))
ext_sync = analyze_dir(os.path.join(os.curdir, "graderbot_tikv", "ext_sync"))

# sort by trial then remaining_workflow_len descending
baseline.sort_values(by=["trial", "remaining_workflow_len"], ascending=[True, False], inplace=True)
ext_sync.sort_values(by=["trial", "remaining_workflow_len"], ascending=[True, False], inplace=True)

# get function completition times relative to first launch time
baseline["net_completed"] = baseline["completed"].sub(baseline.groupby("trial")["launched"].transform("first"))
ext_sync["net_completed"] = ext_sync["completed"].sub(ext_sync.groupby("trial")["launched"].transform("first"))

baseline_function_means = baseline.groupby("function")["net_completed"].mean().sort_values()
baseline_function_std = baseline.groupby("function")["net_completed"].std().reindex(baseline_function_means.index)
ext_sync_function_means = ext_sync.groupby("function")["net_completed"].mean().sort_values()
ext_sync_function_std = ext_sync.groupby("function")["net_completed"].std().reindex(ext_sync_function_means.index)

baseline_function_means.to_csv("graderbot_tikv_baseline.csv")
ext_sync_function_means.to_csv("graderbot_tikv_ext_sync.csv")
pct_improve = (baseline_function_means["graderbot_post_process"] - ext_sync_function_means["graderbot_post_process"]) / baseline_function_means["graderbot_post_process"] * 100
print("External Synchrony Percent Improvement:", pct_improve)

plt.figure(figsize=(10,6))
plt.scatter(np.arange(len(baseline_function_means)), baseline_function_means / 10**6, label="baseline")
plt.errorbar(np.arange(len(baseline_function_means)), baseline_function_means / 10**6, yerr=baseline_function_std / 10**6, fmt="o")
plt.scatter(np.arange(len(ext_sync_function_means)), ext_sync_function_means / 10**6, label="ext\_sync")
plt.errorbar(np.arange(len(ext_sync_function_means)), ext_sync_function_means / 10**6, yerr=ext_sync_function_std / 10**6, fmt="o")
plt.title("External Synchrony vs Baseline on Grading System")
plt.ylabel("Function Completition Time (ms)")
plt.xlabel("Function")
xticks = baseline_function_means.index.map(lambda s : s.replace("_", "\_")).to_list()
xticks[-1] = "post\_process"
plt.xticks(np.arange(len(baseline_function_means)), xticks, fontsize=10)
plt.legend(framealpha=0.5)
plt.savefig("graderbot_tikv.pdf", format="pdf", dpi=600, transparent=True)
plt.show()