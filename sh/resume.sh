#!/bin/bash

pids=$(pgrep -af stride-rs/target | awk '{print $1}')

for pid in $pids; do
  echo "Resuming $pid"
  kill -CONT "$pid"
done
