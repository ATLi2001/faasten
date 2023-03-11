import json
import os
import pandas as pd

# analyze the experiment results in given directory
def analyze_dir(dir_path):
    results = pd.DataFrame(columns=["interop_compute_ms", "runtime (ns)"])
    for f in os.listdir(dir_path):
        curr_experiment = json.load(f)
        # split[-1] give tms.json, want to get t
        interop_compute_ms = f.split("_")[-1][:-6]
        results.loc[len(results)] = [int(interop_compute_ms), curr_experiment["completed"] - curr_experiment["launched"]]

    return results
