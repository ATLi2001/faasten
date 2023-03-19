#!/usr/bin/env bash

ROOTDIR="$HOME/faasten"
GRADERBOTDIR="$HOME/graderbot-functions"

cd $ROOTDIR

# Need to prep the database / blobstore with example submission and grading script
sudo $ROOTDIR/target/debug/sfdb "cos316/example/grading_script" - < $GRADERBOTDIR/output/example_cos316_grader.tgz
sudo $ROOTDIR/target/debug/sfdb "github/cos316/example/submission.tgz" - < $GRADERBOTDIR/output/example_cos316_submission.tgz
sudo $ROOTDIR/target/debug/sfblob < $GRADERBOTDIR/output/example_cos316_grader.tgz | tr -d '\n' | sudo $ROOTDIR/target/debug/sfdb "cos316/example/grading_script" -
sudo $ROOTDIR/target/debug/sfblob < $GRADERBOTDIR/output/example_cos316_submission.tgz | tr -d '\n' | sudo $ROOTDIR/target/debug/sfdb "github/cos316/example/submission.tgz"  -

# Determine the blob store value (ignore the @ symbol)
# this should go into the graderbot workload as args["submission"]
sudo $ROOTDIR/target/debug/sfdb "github/cos316/example/submission.tgz"
