#!/bin/zsh
cd "$(dirname "$0")/.." || exit 1
export RATHERAPIA_RATTY=1
exec cargo run
