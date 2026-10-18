# PROJECT KNOWLEDGE BASE

**Generated:** 2026-09-07T19:52:14+03:00
**Commit:** 90d4531
**Branch:** main

## OVERVIEW
Highwater tracks AI-provider quota windows and pace. Six Rust crates separate daemon state, provider I/O, shared vocabulary, the generated D-Bus proxy, Slint presentation, and `highwaterctl`; Rust edition 2024, MSRV 1.92, Slint 1.18 (FemtoVG: OpenGL on Linux, wgpu on Windows).

## STRUCTURE
```text
highwater/
|-- crates/
|   |-- highwater/        # Slint GUI; consumes IPC, never core
|   |-- highwater-cli/    # highwaterctl; D-Bus client, no core/display/Tokio
|   |-- highwaterd/       # Polling, history, secrets and IPC ownership
|   |-- highwater-core/   # External I/O and provider implementations
|   |-- highwater-ipc/    # Single generated D-Bus client proxy
|   `-- highwater-types/  # Shared domain and wire vocabulary
|-- data/               # Desktop assets, user service, packaging payloads
|-- scripts/            # Layering, integration, packaging and release checks
|-- nix/                # Package and NixOS module
|-- docs/adr/           # Ownership and integration decisions
|-- CONTEXT.md          # Architecture
`-- PLAN.md             # Implementation log
```

## WHERE TO LOOK
| Task | Location | Notes |
|------|----------|-------|
| Add a provider | `crates/highwater-core/src/providers/`, `crates/highwaterd/src/registry.rs` | Core implementation plus daemon registration; simple catalog and hand-written descriptors coexist |
| Change IPC vocabulary | `crates/highwater-types/src/wire.rs` | Keep daemon and GUI compatible |
| Change the D-Bus proxy | `crates/highwater-ipc/src/lib.rs` | Keep the one generated client contract synchronized with the daemon interface |
| Change the CLI | `crates/highwater-cli/` | Preserve JSON/Waybar shapes, exit codes, secret input and layering |
| Polling or mutation ordering | `crates/highwaterd/src/engine.rs`, `service.rs` | Owned state and published mirror |
| GUI or daemon reconnect | `crates/highwater/src/window.rs`, `bus.rs` | Presentation and IPC client |
| Credentials or browser sources | `crates/highwater-core/src/oauth_file.rs`, `secrets.rs`, `browser/` | Ownership and explicit source selection matter |
| Charts or notice identity | `crates/highwater-core/src/storage/` | Segmentation, retention and migrations |
| Installation | `README.md`, `PKGBUILD`, `data/packaging/`, `nix/` | deb/rpm metadata also lives in GUI Cargo manifest |
| Ownership rationale | `CONTEXT.md`, `docs/adr/` | Normative design and binding decisions |
| Release and CI | `scripts/release.sh`, `.github/workflows/` | Release helper commits, tags and pushes |

## CODE MAP
Declarations/import shapes are reported by subtree analysis; exact numeric reference counts are unmeasured.

| Symbol | Type | Location | Refs | Role |
|--------|------|----------|------|------|
| `Engine` / `Command` / `Publication` | Runtime types | `crates/highwaterd/src/engine.rs` | Unmeasured | State, serialized mutations, publication |
| `Daemon` / `Published` | Service types | `crates/highwaterd/src/service.rs` | Unmeasured | D-Bus interface and shared mirror |
| `catalog` / `account` | Functions | `crates/highwaterd/src/registry.rs` | Unmeasured | Definitions and configured clients |
| `Provider` / `ProviderError` | Trait / error | `crates/highwater-core/src/providers/mod.rs` | Unmeasured | Provider contract |
| `Spec` / `HandSpec` / `CATALOG` | Descriptors / catalog | `crates/highwater-core/src/providers/keyed/mod.rs` | Unmeasured | Simple and custom provider definitions |
| `History` | Storage type | `crates/highwater-core/src/storage/mod.rs` | Unmeasured | Persisted usage history |
| `Config` / `CredentialFile` | Configuration / credential types | `crates/highwater-core/src/{config,oauth_file}.rs` | Unmeasured | Preferences and vendor credential updates |
| `ProviderStatus` / `Preferences` | Wire types | `crates/highwater-types/src/wire.rs` | Unmeasured | Cross-process dictionary contract |
| `ids` | Module | `crates/highwater-types/src/lib.rs` | Unmeasured | Installed identity and schema constants |
| `MainWindow` / `AppWindow` | GUI types | `crates/highwater/src/window.rs`, `ui/app.slint` | Unmeasured | Coordination; grid, drag and reorder in markup |
| `DaemonProxy` / `Update` | Client / update types | `crates/highwater/src/bus.rs` | Unmeasured | IPC client and reconnect stream |

**Poll to pixel:** `highwaterd::main::run` loads `Config`, `History`, and `Keyring`; `registry::accounts` supplies accounts. `Engine::poll_due` lazily constructs providers and fetches concurrently; `Engine::apply` calls `History::ingest(&Snapshot)` and `ProviderStatus::set_reading`. The publisher updates the mirror with `Published::upsert`, then emits `Daemon::provider_changed`.
On the GUI side, `bus::watch` drives `DaemonProxy` on Slint's event loop; signals become `Update::Changed`. `MainWindow::handle` calls `show_all`/`show_one`; `view::card` turns a `ProviderStatus` into a `CardView`, and `ui/card.slint` draws the value and pace mark.

**D-Bus contract:** name `io.github.kakadon000.Highwater.Daemon`, path `/io/github/kakadon000/Highwater`, interface `io.github.kakadon000.Highwater.Daemon1`; method `GetStatus`, signal `ProviderChanged`. App ID: `io.github.kakadon000.Highwater`. Activation uses `data/dbus-1/services/` and the systemd user unit `highwaterd.service`. Published `a{sv}` dictionaries are extensible; absent values stay absent.

## CONVENTIONS
- Layering is enforced by `scripts/check-layering.sh`: types have no runtime I/O; IPC has no policy; CLI has no core/HTTP/SQLite/Slint/runtime; core has no Slint/winit/wgpu; GUI has no core/HTTP/SQLite. Types may use zvariant, not zbus.
- Cargo sets `unsafe_code = "deny"`, not `forbid`; audited Windows exceptions exist. Clippy warns on `all`, `todo` and `dbg_macro`.
- Missing provider values stay missing. Window identity is its length, not the vendor field name; slugs are persistent storage keys.
- Config stores preferences, not secrets; preserve TOML decoration and reject present-but-invalid values. Provider/account array order determines UI order.
- Shared proxy policy covers child processes and bypasses loopback; the off setting inherits environment behavior.
- Errors use contextual `thiserror` enums, not `anyhow`. Daemon async runs on manually built multi-thread Tokio; GUI futures run on Slint's event loop; zbus interface calls reach it through `upgrade_in_event_loop`.
- Cargo only; crates declare their own dependencies, without `[workspace.dependencies]`. Change committed `Cargo.lock` through Cargo, never by hand.
- Provider fetch separates transport from pure parsing; a recognized malformed window fails the fetch, while genuinely unknown kinds may be skipped.
- Tests use built-in `#[test]`/`#[tokio::test]`, mostly colocated; integration tests cover process-global behavior and cross-seam flows. Use recorded provider fixtures, deterministic timestamps, `History::in_memory()`, and `FakeSecrets`, not the real keyring.

## ANTI-PATTERNS
- Do not fabricate quota rows, reset timestamps or pace when a provider omits them.
- Do not silently substitute a browser profile/account after a selected source fails, modify live browser databases, or expose credentials in diagnostics.
- Do not create vendor-owned credential files that do not already exist; preserve their ownership boundary.
- Do not configure cargo-deb system-scope `systemd-units` for the user daemon or restart it from RPM postun. Package greeting failure must not fail installation.
- Never run `scripts/release.sh` as a validator: it changes versions and performs commit/tag/push. The core live probe reads a real key and calls Z.ai; exclude it from routine validation.
- Never ship Windows system DLLs; the Inno installer is per-user and unelevated.
- No embedded webview or JS engine. Never rename shipped provider slugs, fabricate history points, or hide windows.
- Root `src/` is ignored scratch, not a Cargo target; `pkg/`, `.worktrees/`, `target/`, and `*.pkg.tar.*` are build output, not editable source.

## UNIQUE STYLES
- Outbound application identity is `Highwater/<version>`, not browser/executable impersonation; T3 Chat separately uses a browser-emulating transport stack.
- Linux uses session D-Bus and systemd user services; Windows uses per-user zbus p2p AF_UNIX, Task Scheduler/HKCU Run, jobs and native tray/toasts.
- The Linux tray uses ksni; Windows uses tray-icon on its own thread. Rubik and the UI icons are compiled into the client; provider marks are runtime assets.
- Provider SVG marks use filled outlines rather than strokes; desktop integration checks enforce this.
- UI is `.slint` markup under `crates/highwater/ui/` drawn with the palette and widgets in `theme.slint` and `widgets.slint`; decisions are pure Rust, state uses `Rc<RefCell<_>>`/`Cell`/`Weak`. FemtoVG blurs text under rounded clips and `opacity` layers, so neither is used. Card and notification thresholds share 70% / 90%.
- Simple API-key providers register alphabetically in `keyed::CATALOG`; unusual auth/multi-request clients use `HandSpec` and daemon `HAND_WRITTEN`, not forced `Keyed` implementations. Provider additions also update README and `docs/TRADEMARKS.md`.

## COMMANDS
```bash
cargo run -p highwaterd
cargo run -p highwater
highwaterctl usage --format json
# Full local gate (plain workspace tests):
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && ./scripts/check-layering.sh
scripts/check-desktop-integration.sh
cargo build --release --locked --workspace
# After building both binaries:
cargo deb --no-build -p highwater
cargo generate-rpm -p crates/highwater
# Packaging/release helper tests:
scripts/test-release.sh
scripts/test-restart-user-daemon.sh
scripts/test-nix-flake.sh
# Public session-bus probes:
busctl --user introspect io.github.kakadon000.Highwater.Daemon /io/github/kakadon000/Highwater
busctl --user call io.github.kakadon000.Highwater.Daemon /io/github/kakadon000/Highwater io.github.kakadon000.Highwater.Daemon1 GetStatus
```

## NOTES
- Commands above are documented entry points, not a claim they passed during knowledge-base generation.
- Secret Service tests skip without a session bus; use plain `cargo test --workspace` for the local gate. Linux validation does not cover Windows-only branches; T3 Chat is Unix-only and Windows Antigravity local agy remains gated.
- Build prerequisites (Debian/Ubuntu): `libfontconfig-dev libsqlite3-dev pkg-config cmake g++ libclang-dev`; Fedora: `fontconfig-devel sqlite-devel pkgconf-pkg-config cmake gcc-c++ clang-devel`. At run time the client opens xkbcommon, the Wayland client (or X11) and libEGL with dlopen.
- T3 Chat's BoringSSL client needs CMake, a C++ compiler, and libclang; `bindgen` generates its bindings at build time. The GUI `build.rs` compiles `.slint` markup and embeds Rubik and UI icons through `slint-build`.
- Release only with `scripts/release.sh X.X.X` from clean, up-to-date `main`. It bumps workspace/dependency versions, lockfile, AppStream release entry, and PKGBUILD; commits, tags, and pushes without running tests. AppStream release prose is human work; tag push starts release CI.
- SQLite is system-linked on Linux and bundled on Windows; TLS uses rustls. Arch packaging disables makepkg LTO for aws-lc-sys.
- CI uses ubuntu-26.04 and native Windows MSVC with a static CRT. Windows packaging pins Inno 7.1.0 by SHA-256 and rejects non-system PE imports.
- Nix exports packages, apps, a NixOS module and a dev shell for x86_64-linux/aarch64-linux.
- `scripts/test-package-upgrade.sh [workdir]` needs Docker/systemd; `scripts/check-tag-version.sh <tag>` validates release version alignment.
- More specific AGENTS.md files cover each crate and core's providers, keyed providers, Antigravity, browser and storage domains; keep implementation details there.

## USER PREFERENCES (binding)
- Never perform manual/UI verification (clicking through the app, screenshots, driving the installed app) — the user does all manual checks. State what to verify by hand and stop.

## BUILDING & TESTING ON THIS WINDOWS MACHINE (binding recipe)
Use PowerShell and the native Visual Studio x64 environment. Visual Studio/Build Tools
must have Desktop development with C++ and a Windows SDK. Do not build through Git Bash
or MSYS2: its coreutils `link.exe` can shadow the MSVC linker.

```powershell
. ./scripts/windows-env.ps1
cargo build --release --locked --target x86_64-pc-windows-msvc -p highwater -p highwaterd --bins
# Or build, validate the payload and compile the pinned Inno setup:
./scripts/build-windows.ps1
```
Always pass the explicit `--target` for builds/checks/tests: `.cargo/config.toml`
sets a static CRT only for target artifacts, leaving host build scripts/proc macros
compatible. Release binaries are in `target/x86_64-pc-windows-msvc/release` and setup
in `build/windows/installer`. See `data/packaging/windows/README.md` for recovery.
Never replace/stop the personal installed app as a validator. Full installer tests
require a clean disposable user and refuse existing Highwater integration; use
`scripts/test-windows-installer.ps1 -CompileOnly` locally. Skip `windows_store`
credential tests against a running personal installation. Live-transport tests can
fail behind a system proxy (HTTP 503); they are not a packaging regression signal.
