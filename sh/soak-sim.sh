#!/bin/bash

read_var() {
  file="$(dirname "$0")"/.env.soak-sim
  if [ ! -f "$file" ]; then
    exit 1
  fi
  var=$(grep "^$1=" "$file" | cut -d '=' -f2)
  echo "$var"
}

set -e
if [ "$THREADS" == "" ]; then
  THREADS=$(read_var THREADS)
  export THREADS
fi

if [ "$SCALE" == "" ]; then
  SCALE=$(read_var SCALE)
  export SCALE
fi

if [ "$RUST_LOG" == "" ]; then
  RUST_LOG=$(read_var RUST_LOG)
  export RUST_LOG
fi

SOAK_CMD="cargo test _model::sim_ --release -- --nocapture --test-threads $THREADS" "$(dirname "$0")"/soak.sh
