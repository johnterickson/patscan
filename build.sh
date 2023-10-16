#!/bin/bash

set -e
pushd "$(dirname "$0")"
pushd rs
cargo build --release
cargo test
cargo test --release
cargo bench
popd
pushd c-sharp/test
dotnet test -l "console;verbosity=detailed"
popd
pushd c-sharp/bench
dotnet run -c Release
popd
popd