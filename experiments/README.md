# Setup
### [CloudLab](https://cloudlab.us/) Setup

1. Run experiment (Praxis project, snapfaas-cluster profile, no other options)
2. Use node1 only (do not use `couchdb` node)
3. Install nix, need to restart after
```
sh <(curl -L https://nixos.org/nix/install) --daemon
```
4. Add nixpkgs
```
nix-channel --add https://nixos.org/channels/nixos-22.05 nixpkgs
nix-channel --update
```
5. Clone ATLi2001/faasten
```
git clone https://github.com/ATLi2001/faasten.git
```
6. Activate nix-shell
7. Give docker permissions to self, need to restart after
```
sudo groupadd docker
sudo usermod -aG docker $USER
```
8. Make root image
```
./mk_rtimage.sh python3 python3.ext4
```
9. `cargo build --release`

For graderbot-functions
1. Clone ATLi2001/graderbot-functions
```
git clone https://github.com/ATLi2001/graderbot-functions.git
```
3. Activate nix-shell 
4. `make`
5. `make output/example_cos316_grader.tgz output/example_cos316_submission.tgz`

### [TiKV](https://tikv.org/) Setup
This will create a local TiKV cluster that listens at `public_ip_address`
```
curl --proto '=https' --tlsv1.2 -sSf https://tiup-mirrors.pingcap.com/install.sh | sh
source .bashrc
tiup
tiup update --self && tiup update playground
nohup tiup playground --mode tikv-slim --host public_ip_address &
```

# Run Experiments
For all experiments, run one time as external synchrony (`ext_sync`) and one time as baseline. 
For the baseline runs, need to change `db_client.rs` to always have `self.send_to_background_thread(sc, true)`.
Have the TiKV server running on some `public_ip_address`, and put that address into `distributed_db/mod.rs`.

### Synthetic
Fixed reps, varying interop delay time. Set REPS in `run_experiment_synthetic.sh`
```
nohup bash run_experiment_synthetic.sh ext_sync tikv_interop 0 200 5 &
```

Fixed interop delay time, varying reps. Set INTEROP_COMPUTE_MS in `run_experiment_synthetic.sh`
```
nohup bash run_experiment_synthetic.sh ext_sync tikv_reps 1 1 1 &
nohup bash run_experiment_synthetic.sh ext_sync tikv_reps 0 150 5 &
```

### Graderbot
```
bash prep_experiment_graderbot.sh
nohup bash run_experiment_graderbot.sh ext_sync &
```
