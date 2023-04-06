#!/usr/bin/env bash

ROOTDIR="$HOME/faasten"
GRADERBOTDIR="$HOME/graderbot-functions"

cd $ROOTDIR

# Need to prep the database / blobstore with example submission and grading script
sudo $ROOTDIR/target/release/sfdb "cos316/example/grading_script" - < $GRADERBOTDIR/output/example_cos316_grader.tgz
sudo $ROOTDIR/target/release/sfdb "github/cos316/example/submission.tgz" - < $GRADERBOTDIR/output/example_cos316_submission.tgz
sudo $ROOTDIR/target/release/sfblob < $GRADERBOTDIR/output/example_cos316_grader.tgz | tr -d '\n' | sudo $ROOTDIR/target/release/sfdb "cos316/example/grading_script" -
sudo $ROOTDIR/target/release/sfblob < $GRADERBOTDIR/output/example_cos316_submission.tgz | tr -d '\n' | sudo $ROOTDIR/target/release/sfdb "github/cos316/example/submission.tgz"  -

# Determine the blob store value (ignore the @ symbol)
# this should go into the graderbot workload as args["submission"]
# use sed '$d' to get rid of extra stdout line from tikv client
blobstore_val=$(sudo $ROOTDIR/target/release/sfdb "github/cos316/example/submission.tgz" | sed '$d' | xargs)
blobstore_val=${blobstore_val:1}
echo $blobstore_val

graderbot_workload_json="{\"args\": {\"submission\": \"$blobstore_val\"}, \"workflow\": [ \"go_grader\", \"grades\", \"generate_report\", \"graderbot_post_process\" ], \"context\": { \"repository\": \"cos316/example/\", \"commit\": \"b541c851d79edad1d05fc64c1bcca88800703a30\", \"push_date\": 1642798607, \"metadata\": {\"assignment\": \"example\"}}}"
echo $graderbot_workload_json > "$ROOTDIR/graderbot_pre_post_process/graderbot_pre_process/graderbot_workload.json"
