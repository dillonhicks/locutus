#!/usr/bin/env bash
set -exo pipefail

cargo build -p locutus-server
cargo run -p locutus-server -- \
      --log-level info \
      --host 127.0.0.1 \
      --port 9001 \
      --tick-hertz 30
