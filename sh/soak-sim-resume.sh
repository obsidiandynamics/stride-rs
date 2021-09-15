#!/bin/bash

pids=$(pgrep -af stride-rs)

for pid in $pids; do
  echo "Resuming $pid"
  kill -CONT "$pid"
done