pushd %~dp0
pushd rs
cargo build --release
popd
pushd c-sharp\test
dotnet test -l "console;verbosity=detailed"
popd
popd