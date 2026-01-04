# Watcher simple_case Fixture

`compiler/runtime/tests/watcher.rs` uses this directory as the canonical
input for end-to-end watcher tests. The file `initial.txt` represents a static
payload that is copied into a temporary directory before the watcher starts.
The test then modifies and removes the copied file to ensure `WatchEvent`
emission stays stable across platforms.
