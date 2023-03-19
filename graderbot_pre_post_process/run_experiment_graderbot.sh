#!/usr/bin/env bash

ROOTDIR="$HOME/faasten"
GRADERBOTDIR="$HOME/graderbot-functions"
OUTDIR="$ROOTDIR/out"
RESULTDIR="$ROOTDIR/experiments/graderbot"

if [ $# -ne 1 ]; then
    echo 'usage: ./run_experiment_graderbot.sh NAME'
    exit 1
fi

make -f Makefile
# pre, post process application images needed to be moved 
cp "./graderbot_pre_process/output.ext2" "$GRADERBOTDIR/output/graderbot_pre_process.img"
cp "./graderbot_post_process/output.ext2" "$GRADERBOTDIR/output/graderbot_post_process.img"

# clear OUTDIR
sudo rm -f $OUTDIR/*

# make RESULTDIR
mkdir -p $RESULTDIR/$1

cd $ROOTDIR

# run multivm in background
sudo $ROOTDIR/target/debug/multivm --config $ROOTDIR/resources/graderbot-config.yaml --mem 1024 --listen 127.0.0.1:3456 &

# sleep so multivm can have time to start listen
sleep 1

# run sfclient
sudo $ROOTDIR/target/debug/sfclient -s 127.0.0.1:3456 -f graderbot_pre_process < $ROOTDIR/graderbot_pre_post_process/graderbot_pre_process/graderbot_workload.json

# because sfclient returns upon the first function completion
# sleep to allow rest of functions to run
sleep 60

# send ^C to multivm
sudo kill -INT $(pidof multivm)

# collect data
i=0
for outfile in "$OUTDIR"/* 
do
    # only copy over non empty files
    if [ -s "$outfile" ]; then 
        cp $outfile "$RESULTDIR/${1}/go_grader_grades_generate_report_${i}.json"
        i=$i+1
    fi
done