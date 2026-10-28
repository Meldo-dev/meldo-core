# meldo-core

The shared Rust core of [Meldo](https://meldo.dev), the school planner for students in Germany. It holds everything both apps compute the same way: grade averages on the 1–6 and 0–15 scales, timetable derivation (A/B weeks, substitutions), WebUntis parsing, participation statistics and task rules. It's exposed to Swift and Kotlin through [UniFFI](https://github.com/mozilla/uniffi-rs).

| Crate | What it does |
|---|---|
| `melde-core` | Shared types: dates, subjects, lessons |
| `melde-grades` | Grade scales, weighting, rounding, "grade needed" |
| `melde-scheduling` | Timetable, A/B weeks, bell schedule |
| `melde-statistics` | Participation counts, streaks, weekly goals |
| `melde-tasks` | Homework and to-do rules, recurrence |
| `melde-untis` | WebUntis JSON-RPC/REST parsing and derivation |
| `melde-ffi` | The UniFFI surface both apps bind to |
| `tools/uniffi-bindgen` | Pinned binding generator |

`l10n/Localizable.xcstrings` is the string catalog (German and English) shared by both apps.

## Use

The apps include this repo as a git submodule at `core/`:

- [meldo-ios](https://github.com/meldo-dev/meldo-ios): `scripts/build-rust-ios.sh` builds an XCFramework and the Swift bindings.
- [meldo-android](https://github.com/meldo-dev/meldo-android): Gradle builds the `.so` libraries with `cargo-ndk` and the Kotlin bindings.

## Develop

```bash
cargo test --workspace
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

After changing the core, bump the submodule in both app repos.

## License

[MIT](LICENSE)
