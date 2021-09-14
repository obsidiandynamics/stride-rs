#!/bin/bash

if [ "$THREADS" == "" ]; then
  export THREADS=1
fi

if [ "$SCALE" == "" ]; then
  export SCALE=1000
fi

if [ "$RUST_LOG" == "" ]; then
  export RUST_LOG=debug
fi

SOAK_CMD="cargo test _model::sim_ --release -- --nocapture --test-threads $THREADS" "$(dirname $0)"/soak.sh
