# IPC CONTRACT KNOWLEDGE BASE

## OVERVIEW
The generated D-Bus proxy for `io.github.kakadon000.Highwater.Daemon1`, shared by every client: the desktop client and `highwaterctl` build their proxy from the same trait, so a method that changed shape breaks the build instead of a user's machine.

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Method, property or signal set | `src/lib.rs` | The one `#[zbus::proxy]` trait; `highwaterd/src/service.rs` is the server half |
| Wire payload shapes | `../highwater-types/src/wire.rs` | This crate names those types, never defines them |
| Interface name, path, bus name | `../highwater-types/src/lib.rs` (`ids`) | The proxy attribute repeats them as literals; the test pins the pair |

## CONVENTIONS
- `#[zbus::proxy]` consumes the trait: the public names are `DaemonProxy` and one message struct per signal (`ProviderChanged`, `DataChanged`, …). There is no `Daemon` trait at runtime.
- Signatures mirror the daemon's interface exactly; a change here requires the same change in `highwaterd`'s `#[zbus::interface]`.
- `zbus`'s `p2p` feature stays enabled: the Windows GUI builds a peer-to-peer connection and generates its proxy from this crate.
- Nothing here opens a connection, retries, or names a transport — reconnection is `highwater/src/bus.rs`, and the CLI's single connection is `highwater-cli/src/connect.rs`.

## ANTI-PATTERNS
- Do not add provider I/O, storage, a display dependency, or a second async runtime; `scripts/check-layering.sh` forbids `highwater-core`, `reqwest`, `hyper`, `rusqlite`, `libsqlite3-sys`, `slint` and `tokio`.
- Do not define wire structures here; they belong to `highwater-types` so the daemon and clients decode one vocabulary.
- Never let a second `#[zbus::proxy]` definition exist anywhere in the workspace: every
  client generates from `src/lib.rs`, so interface drift is a compile failure.
- Do not add a client-side policy (defaults on error, retry, fallback) to the contract: a caller that wants one writes it.

## CHECKS
- `cargo test -p highwater-ipc` serves a fake daemon on the session bus and reads it back; with no session bus it prints `skipped: no session bus reachable` and passes.
- `cargo test -p highwater` exercises the same proxy through the window's reconnect paths.
