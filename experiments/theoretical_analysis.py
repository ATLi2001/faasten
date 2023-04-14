import numpy as np
import matplotlib.pyplot as plt

plt.rcParams.update({"text.usetex": True})
plt.style.use("fivethirtyeight")

execution_time = np.arange(150)
write_time = 50

baseline_time = execution_time + write_time
ext_sync_time = np.maximum(execution_time, write_time)

pct_improve = (baseline_time - ext_sync_time) / baseline_time * 100

plt.figure(figsize=(10,6))
plt.plot(execution_time, pct_improve, label="Write Time = 50")
plt.title("External Synchrony Theoretical Improvement")
plt.ylabel("Percent Improvment")
plt.xlabel("Execution Time")
plt.legend(framealpha=0.5)
plt.tight_layout()
plt.savefig("theoretical.pdf", format="pdf", dpi=600, transparent=True)
plt.show()