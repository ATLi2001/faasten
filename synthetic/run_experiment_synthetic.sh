#!/usr/bin/env bash

ROOTDIR="$HOME/faasten"
OUTDIR="$ROOTDIR/out"
RESULTDIR="$ROOTDIR/experiments/synthetic/single"

if [ $# -ne 5 ]; then
    echo 'usage: ./run_experiment_synthetic.sh NAME REPS MIN_MS MAX_MS STEP'
    exit 1
fi

make -f Makefile

# clear OUTDIR
sudo rm -f $OUTDIR/*

# make RESULTDIR
mkdir -p $RESULTDIR/$1

# for each time from MIN_MS to MAX_MS, incrementing by STEP each time
t=$3
while [ $t -le $4 ]; do
    echo $t 
    # run synthetic workload 
    bash run_single_synthetic.sh $2 $t
    # collect data
    for outfile in "$OUTDIR"/* 
    do
	# only copy over non empty files
        if [ -s "$outfile" ]; then 
            cp $outfile "$RESULTDIR/${1}/synthetic_${2}reps_${t}ms.json"
        fi
    done

    sudo rm -f $OUTDIR/*
    
    t=$(($t+$5))
done
