#!/usr/bin/env bash

ROOTDIR="$HOME/faasten"
GRADERBOTDIR="$HOME/graderbot-functions"
OUTDIR="$ROOTDIR/out"
RESULTDIR="$ROOTDIR/experiments/graderbot"
TRIALS=50

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

# Determine the blob store value (ignore the @ symbol)
# this should go into the graderbot workload as args["submission"]
blobstore_val=$(sudo $ROOTDIR/target/debug/sfdb "github/cos316/example/submission.tgz" | xargs)
blobstore_val=${blobstore_val:1}

# run multivm in background
sudo $ROOTDIR/target/debug/multivm --config $ROOTDIR/resources/graderbot_config.yaml --mem 1024 --listen 127.0.0.1:3456 &

# sleep so multivm can have time to start listen
sleep 1

# run sfclient
sudo $ROOTDIR/target/debug/sfclient -s 127.0.0.1:3456 -f graderbot_pre_process < $ROOTDIR/graderbot_pre_post_process/graderbot_pre_process/graderbot_workload.json

# because sfclient returns upon the first function completion
# sleep to allow rest of functions to run
sleep 20

# clear OUTDIR
sudo rm -f $OUTDIR/*

# trials loop for now warmed up system
for (( i=0; i<$TRIALS; i++))
do 
    echo "trial $i"

    # warmed up system can start from go_grader directly, no need for the pre process
    # make sure context has trial number in it
    graderbot_workload_json="{\"args\": {\"submission\": \"$blobstore_val\"}, \"workflow\": [ \"grades\", \"generate_report\", \"graderbot_post_process\" ], \"context\": { \"repository\": \"cos316/example/\", \"commit\": \"b541c851d79edad1d05fc64c1bcca88800703a30\", \"push_date\": 1642798607, \"metadata\": {\"assignment\": \"example\", \"trial\": $i}}}"
    echo $graderbot_workload_json > "$ROOTDIR/graderbot_pre_post_process/graderbot_warm_workload.json"

    # run sfclient starting at go_grader
    sudo $ROOTDIR/target/debug/sfclient -s 127.0.0.1:3456 -f go_grader < $ROOTDIR/graderbot_pre_post_process/graderbot_warm_workload.json

    # because sfclient returns upon the first function completion
    # sleep to allow rest of functions to run
    sleep 20
done

# send ^C to multivm
sudo kill -INT $(pidof multivm)

# collect data
i=0
for outfile in "$OUTDIR"/* 
do
    # only copy over non empty files
    if [ -s "$outfile" ]; then 
        cp $outfile "$RESULTDIR/${1}/go_grader_grades_generate_report_${i}.json"
	i=$((i+1))
    fi
done
