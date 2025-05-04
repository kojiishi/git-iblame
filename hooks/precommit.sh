#!/bin/bash
set -e
(
  set -x
  cargo test --all-features
)
if [[ "$1" == '-n' ]]; then
  set -x
  cargo clippy --all-targets --all-features -- -D warnings
  cargo fmt --all --check
else
  set -x
  cargo clippy --fix --allow-dirty --all-targets --all-features -- -D warnings
  cargo fmt --all
fi
