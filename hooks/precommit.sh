#!/bin/bash
set -e
(
  set -x
  cargo test --all-targets --all-features
)
if [[ "$1" == '-n' ]]; then
  set -x
  cargo fmt --all --check
  cargo clippy --all-targets --all-features -- -D warnings
else
  set -x
  cargo fmt --all
  cargo clippy --fix --allow-dirty --all-targets --all-features -- -D warnings
fi
