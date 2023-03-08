#!/usr/bin/env bash

ROOTDIR="/home/atli/faasten"

if [ $# -ne 2 ]; then
    echo 'usage: ./run_synthetic.sh REPS INTEROP_COMPUTE_MS'
    exit 1
fi

# synthetic_workload.json file
echo "{\"args\": {\"reps\": $1, \"interop_compute_ms\": $2}, \"workflow\": [ ], \"context\": { }}" > synthetic_workload.json

# run multivm in background
sudo RUST_LOG=debug $ROOTDIR/target/debug/multivm --config $ROOTDIR/synthetic/synthetic.yaml --mem 1024 --listen 127.0.0.1:3456 &
PID=$!

# run sfclient
sudo RUST_LOG=debug $ROOTDIR/target/debug/sfclient -s 127.0.0.1:3456 -f synthetic < $ROOTDIR/synthetic/synthetic_workload.json

# send ^C to multivm
kill -INT $PID
