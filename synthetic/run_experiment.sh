#!/usr/bin/env bash

ROOTDIR="~/faasten"
OUTDIR="~/faasten/out"
RESULTDIR="~/faasten/experiments/synthetic/single"

if [ $# -ne 4 ]; then
    echo 'usage: ./run_synthetic.sh MIN_MS MAX_MS STEP REPS'
    exit 1
fi

t=$1
while [ $t -le $2 ]; do
    echo $t 
    # run synthetic workload 
    run_synthetic.sh $4 $t
    # collect data
    for outfile in "$OUTDIR"/* 
    do 
        if [ -s "$outfile" ]; then 
            cp "$OUTDIR/$outfile" "$RESULTDIR/synthetic_${4}reps_${t}ms.json"
        fi
    done

    t=$(($t+$3))
done
