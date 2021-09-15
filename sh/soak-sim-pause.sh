#!/bin/bash

pids=$(pgrep -af stride-rs)

for pid in $pids; do
  echo "Suspending $pid"
  kill -STOP "$pid"
done