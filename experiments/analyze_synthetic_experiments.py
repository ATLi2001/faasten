import json
import os
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt

# analyze the experiment results in given directory
def analyze_dir(dir_path):
    results = pd.DataFrame(columns=["reps", "interop_compute_ms", "globaldb_ms", "trial", "runtime_ns"])
    for file_path in os.listdir(dir_path):
        print(file_path)
        with open(os.path.join(dir_path, file_path), "r") as f:
            curr_experiment = json.load(f)

            args = file_path.split("_")
            reps = int(args[1][:-4])
            interop_compute_ms = int(args[2][7:-2])
            globaldb_ms = int(args[3][8:-2])
            trial = int(args[4][5:-5])
            results.loc[len(results)] = [
                reps, 
                interop_compute_ms, 
                globaldb_ms,
                trial,
                curr_experiment["completed"] - curr_experiment["launched"]
            ]

    return results

def analyze_ext_sync_baseline(df_ext_sync, df_baseline, name):
    if name == "interop":
        index_order = ["reps", "interop_compute_ms", "trial"]
        groupby_order = ["reps", "interop_compute_ms"]
        label = "reps"

    if name == "reps":
        index_order = ["interop_compute_ms", "reps", "trial"]
        groupby_order = ["interop_compute_ms", "reps"]
        label = "interop ms"
    
    # sort and set index
    df_ext_sync.sort_values(by=index_order, inplace=True)
    df_baseline.sort_values(by=index_order, inplace=True)


    df_ext_sync.set_index(index_order, inplace=True)
    df_baseline.set_index(index_order, inplace=True)
    
    common_index = df_ext_sync.index.intersection(df_baseline.index)
    df_ext_sync = df_ext_sync.loc[common_index]
    df_baseline = df_baseline.loc[common_index]

    df_pct_improve = df_ext_sync.copy()
    df_pct_improve["pct_improve"] = (df_baseline["runtime_ns"] - df_ext_sync["runtime_ns"]) / df_baseline["runtime_ns"] * 100

    df_pct_improve_mean = df_pct_improve.groupby(groupby_order)["pct_improve"].mean()
    df_pct_improve_std = df_pct_improve.groupby(groupby_order)["pct_improve"].std()

    print(df_pct_improve_mean)
    print(df_pct_improve_std)

    for index_0_val in pd.unique(df_pct_improve_mean.index.get_level_values(0)):
        curr_df = df_pct_improve_mean[df_pct_improve_mean.index.get_level_values(0) == index_0_val]
        plt.scatter(
            curr_df.index.get_level_values(1), 
            curr_df, 
            label="%d %s" % (index_0_val, label),
        )
        plt.errorbar(
            curr_df.index.get_level_values(1), 
            curr_df, 
            yerr=df_pct_improve_std[df_pct_improve_std.index.get_level_values(0) == index_0_val],
            fmt="o",
        )
    
    plt.title("External Synchrony Improvement vs Interop Delay")
    plt.ylabel("Percent Improvment")
    plt.xlabel("Inter Operation Delay (ms)")
    plt.legend()
    plt.show()


synthetic_experiments = {
    "synthetic_ext_sync_interop": {
        "results_path": os.path.join(os.curdir, "synthetic", "ext_sync", "interop"),
        "csv_path": "synthetic_ext_sync_interop.csv"
    }, 
    "synthetic_baseline_interop": {
        "results_path": os.path.join(os.curdir, "synthetic", "baseline", "interop"),
        "csv_path": "synthetic_baseline_interop.csv"
    },
    "synthetic_ext_sync_reps": {
        "results_path": os.path.join(os.curdir, "synthetic", "ext_sync", "reps"),
        "csv_path": "synthetic_ext_sync_reps.csv"
    },
    "synthetic_baseline_reps": {
        "results_path": os.path.join(os.curdir, "synthetic", "baseline", "reps"),
        "csv_path": "synthetic_baseline_reps.csv"
    }
}

for experiment in synthetic_experiments.keys():
    experiment_data = synthetic_experiments[experiment]
    if os.path.isfile(experiment_data["csv_path"]):
        experiment_data["df"] = pd.read_csv(experiment_data["csv_path"], index_col=0)
    else:
        experiment_data["df"] = analyze_dir(experiment_data["results_path"])
        experiment_data["df"].to_csv(experiment_data["csv_path"])
    synthetic_experiments[experiment] = experiment_data

analyze_ext_sync_baseline(
    synthetic_experiments["synthetic_ext_sync_interop"]["df"], 
    synthetic_experiments["synthetic_baseline_interop"]["df"], 
    "interop"
)
analyze_ext_sync_baseline(
    synthetic_experiments["synthetic_ext_sync_reps"]["df"], 
    synthetic_experiments["synthetic_baseline_reps"]["df"], 
    "reps"
)
