#!/usr/bin/env bash

ROOTDIR="/home/atli/faasten"
OUTDIR="$ROOTDIR/out"
RESULTDIR="$ROOTDIR/experiments/synthetic/single"

if [ $# -ne 4 ]; then
    echo 'usage: ./run_experiment_synthetic.sh REPS MIN_MS MAX_MS STEP'
    exit 1
fi

make -f Makefile

t=$2
while [ $t -le $3 ]; do
    echo $t 
    # run synthetic workload 
    run_single_synthetic.sh $1 $t
    # collect data
    for outfile in "$OUTDIR"/* 
    do 
        if [ -s "$outfile" ]; then 
            cp "$OUTDIR/$outfile" "$RESULTDIR/synthetic_${1}reps_${t}ms.json"
        fi
    done

    t=$(($t+$4))
done
