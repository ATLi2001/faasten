#!/usr/bin/env bash

ROOTDIR="$HOME/faasten"
OUTDIR="$ROOTDIR/out"
RESULTDIR="$ROOTDIR/experiments/synthetic"
TRIALS=100
REPS=100
INTEROP_COMPUTE_MS=50
GLOBAL_DB_DELAY_MS=50

if [ $# -ne 5 ]; then
    echo 'usage: ./run_experiment_synthetic.sh PROTOCOL_NAME EXPERIMENT_NAME MIN MAX STEP'
    exit 1
fi

# need to have EXPERIMENT_NAME correct
if [ $2 != "reps" && $2 != "interop"]; then 
    echo "EXPERIMENT_NAME incorrect"
    exit 1
fi

make -f Makefile

# clear OUTDIR
sudo rm -f $OUTDIR/*

# make RESULTDIR
mkdir -p $RESULTDIR/$1/$2

# trials loop
for (( i=0; i<$TRIALS; i++))
do 
    # for each time from MIN to MAX, incrementing by STEP each time
    x=$3
    while [ $x -le $4 ]; do
        echo $x

        # run synthetic workload 
        # if EXPERIMENT_NAME is reps, then we are varying reps and keeping interop constant
        # if EXPERIMENT_NAME is interop, then we are varying interop and keeping reps constant
        FILENAME="temp.json"
        if [ $2 = "reps" ]; then
            bash run_single_synthetic.sh $x $INTEROP_COMPUTE_MS
            FILENAME="synthetic_${x}reps_interop${INTEROP_COMPUTE_MS}ms_globaldb${GLOBAL_DB_DELAY_MS}ms_trial${i}.json"
        else
            bash run_single_synthetic.sh $REPS $x
            FILENAME="synthetic_${REPS}reps_interop${x}ms_globaldb${GLOBAL_DB_DELAY_MS}ms_trial${i}.json"
        fi

        # collect data
        for outfile in "$OUTDIR"/* 
        do
        # only copy over non empty files
            if [ -s "$outfile" ]; then 
                cp $outfile "$RESULTDIR/${1}/$FILENAME"
            fi
        done

        sudo rm -f $OUTDIR/*
        
        x=$(($x+$5))
    done
done
