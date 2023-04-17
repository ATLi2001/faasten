import numpy as np
import matplotlib
import matplotlib.pyplot as plt

matplotlib.use("pgf")
plt.rcParams.update({
    "pgf.texsystem": "pdflatex",
    "font.family": "serif",
    "text.usetex": True,
    "pgf.rcfonts": False,
})
plt.style.use("fivethirtyeight")

execution_time = np.arange(200)
write_time = 100

baseline_time = execution_time + write_time
ext_sync_time = np.maximum(execution_time, write_time)

pct_improve = (baseline_time - ext_sync_time) / baseline_time * 100

plt.figure(figsize=(10,6))
plt.plot(execution_time, pct_improve, label="Write Time = %d" % write_time)
plt.title("External Synchrony Theoretical Improvement")
plt.ylabel("Percent Improvement")
plt.xlabel("Execution Time")
plt.legend(framealpha=0.5)
plt.tight_layout()
plt.savefig("theoretical.pdf", format="pdf", dpi=600, transparent=True)
# plt.show()