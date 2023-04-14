import json
import os
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt

plt.rcParams.update({"text.usetex": True})
plt.style.use("fivethirtyeight")

# analyze the experiment results in given directory
def analyze_dir(dir_path):
    results = pd.DataFrame(columns=["reps", "interop_compute_ms", "trial", "runtime_ns"])
    for file_path in os.listdir(dir_path):
        print(file_path)
        with open(os.path.join(dir_path, file_path), "r") as f:
            curr_experiment = json.load(f)

            args = file_path.split("_")
            reps = int(args[1][:-4])
            interop_compute_ms = int(args[2][7:-2])
            trial = int(args[4][5:-5])
            results.loc[len(results)] = [
                reps, 
                interop_compute_ms, 
                trial,
                curr_experiment["completed"] - curr_experiment["launched"]
            ]

    return results

def analyze_ext_sync_baseline(df_ext_sync, df_baseline, name):
    if name == "interop":
        index_order = ["reps", "interop_compute_ms", "trial"]
        groupby_order = ["reps", "interop_compute_ms"]
        label = "reps"
        xlabel = "Interoperation Delay (ms)"

    if name == "reps":
        index_order = ["interop_compute_ms", "reps", "trial"]
        groupby_order = ["interop_compute_ms", "reps"]
        label = "interop ms"
        xlabel = "Reps"

    if name == "globaldb":
        index_order = ["globaldb_ms", "trial"]
        groupby_order = ["globaldb_ms"]
        label = "globaldb ms"
        xlabel = "Global Db Delay (ms)"

    def drop_outliers(df):
        initial_len = len(df)
        max_drop_per_group = 0
        for index, df_group in df.groupby(groupby_order):
            q_low = df_group["runtime_ns"].quantile(0.1)
            q_high = df_group["runtime_ns"].quantile(0.9)
            iqr = q_high-q_low
            low_drop = df_group["runtime_ns"] < (q_low - 1.5*iqr) 
            high_drop = df_group["runtime_ns"] > (q_high + 1.5*iqr)
            if len(df_group[(low_drop | high_drop)]) > 0:
                if len(df_group[(low_drop | high_drop)]) > max_drop_per_group:
                    max_drop_per_group = len(df_group[(low_drop | high_drop)])
                df.drop(df_group[(low_drop | high_drop)].index, inplace=True)
        final_len = len(df)
        print("Total dropped:", initial_len - final_len)
        print("Max dropped per group:", max_drop_per_group)
        return df
    
    # sort and set index
    df_ext_sync.sort_values(by=index_order, inplace=True)
    df_baseline.sort_values(by=index_order, inplace=True)

    print(df_ext_sync)
    print(df_baseline)

    df_ext_sync.set_index(index_order, inplace=True)
    df_baseline.set_index(index_order, inplace=True)

    df_ext_sync = drop_outliers(df_ext_sync)
    df_baseline = drop_outliers(df_baseline)
    
    common_index = df_ext_sync.index.intersection(df_baseline.index)
    df_ext_sync = df_ext_sync.loc[common_index]
    df_baseline = df_baseline.loc[common_index]

    df_pct_improve = df_ext_sync.copy()
    df_pct_improve["pct_improve"] = (df_baseline["runtime_ns"] - df_ext_sync["runtime_ns"]) / df_baseline["runtime_ns"] * 100

    df_pct_improve_mean = df_pct_improve.groupby(groupby_order)["pct_improve"].mean()
    df_pct_improve_std = df_pct_improve.groupby(groupby_order)["pct_improve"].std()

    print(df_pct_improve_mean)
    print(df_pct_improve_std)

    plt.figure(figsize=(10,6))

    if len(groupby_order) > 1:
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
    else:
        plt.scatter(
            df_pct_improve_mean.index,
            df_pct_improve_mean,
        )
        plt.errorbar(
            df_pct_improve_mean.index,
            df_pct_improve_mean,
            yerr=df_pct_improve_std,
            fmt="o",
        )
    
    plt.title("External Synchrony Improvement vs %s" % xlabel)
    plt.ylabel("Percent Improvment")
    plt.xlabel(xlabel)
    plt.legend(framealpha=0.5)
    plt.tight_layout()
    plt.savefig("synthetic_tikv_{}.pdf".format(name), format="pdf", dpi=600, transparent=True)
    plt.show()


synthetic_experiments = {
    # "synthetic_ext_sync_interop": {
    #     "results_path": os.path.join(os.curdir, "synthetic", "ext_sync", "interop"),
    #     "csv_path": "synthetic_ext_sync_interop.csv"
    # }, 
    # "synthetic_baseline_interop": {
    #     "results_path": os.path.join(os.curdir, "synthetic", "baseline", "interop"),
    #     "csv_path": "synthetic_baseline_interop.csv"
    # },
    # "synthetic_ext_sync_reps": {
    #     "results_path": os.path.join(os.curdir, "synthetic", "ext_sync", "reps"),
    #     "csv_path": "synthetic_ext_sync_reps.csv"
    # },
    # "synthetic_baseline_reps": {
    #     "results_path": os.path.join(os.curdir, "synthetic", "baseline", "reps"),
    #     "csv_path": "synthetic_baseline_reps.csv"
    # },
    # "synthetic_ext_sync_globaldb": {
    #     "results_path": os.path.join(os.curdir, "synthetic", "ext_sync", "globaldb"),
    #     "csv_path": "synthetic_ext_sync_globaldb.csv"
    # },
    # "synthetic_baseline_globaldb": {
    #     "results_path": os.path.join(os.curdir, "synthetic", "baseline", "globaldb"),
    #     "csv_path": "synthetic_baseline_globaldb.csv"
    # },
    "synthetic_ext_sync_tikv_interop": {
        "results_path": os.path.join(os.curdir, "synthetic", "ext_sync", "tikv_interop"),
        "csv_path": "synthetic_ext_sync_tikv_interop.csv"
    }, 
    "synthetic_baseline_tikv_interop": {
        "results_path": os.path.join(os.curdir, "synthetic", "baseline", "tikv_interop"),
        "csv_path": "synthetic_baseline_tikv_interop.csv"
    },
    "synthetic_ext_sync_tikv_reps": {
        "results_path": os.path.join(os.curdir, "synthetic", "ext_sync", "tikv_reps"),
        "csv_path": "synthetic_ext_sync_tikv_reps.csv"
    },
    "synthetic_baseline_tikv_reps": {
        "results_path": os.path.join(os.curdir, "synthetic", "baseline", "tikv_reps"),
        "csv_path": "synthetic_baseline_tikv_reps.csv"
    },
}

for experiment in synthetic_experiments.keys():
    experiment_data = synthetic_experiments[experiment]
    if os.path.isfile(experiment_data["csv_path"]):
        experiment_data["df"] = pd.read_csv(experiment_data["csv_path"], index_col=0)
    else:
        experiment_data["df"] = analyze_dir(experiment_data["results_path"])
        experiment_data["df"].to_csv(experiment_data["csv_path"])
    synthetic_experiments[experiment] = experiment_data

# analyze_ext_sync_baseline(
#     synthetic_experiments["synthetic_ext_sync_interop"]["df"], 
#     synthetic_experiments["synthetic_baseline_interop"]["df"], 
#     "interop"
# )
# analyze_ext_sync_baseline(
#     synthetic_experiments["synthetic_ext_sync_reps"]["df"], 
#     synthetic_experiments["synthetic_baseline_reps"]["df"], 
#     "reps"
# )
# analyze_ext_sync_baseline(
#     synthetic_experiments["synthetic_ext_sync_globaldb"]["df"], 
#     synthetic_experiments["synthetic_baseline_globaldb"]["df"], 
#     "globaldb"
# )
analyze_ext_sync_baseline(
    synthetic_experiments["synthetic_ext_sync_tikv_interop"]["df"], 
    synthetic_experiments["synthetic_baseline_tikv_interop"]["df"], 
    "interop"
)
analyze_ext_sync_baseline(
    synthetic_experiments["synthetic_ext_sync_tikv_reps"]["df"], 
    synthetic_experiments["synthetic_baseline_tikv_reps"]["df"], 
    "reps"
)
