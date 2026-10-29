# meldo-core

This is the Rust core of [Meldo](https://meldo.dev), a school planner for students in Germany. Anything the iOS and Android apps must compute the same way lives here: grade averages on the 1–6 and 0–15 scales, the timetable with A/B weeks and substitutions, participation statistics and the rules for tasks. Both apps call it through [UniFFI](https://github.com/mozilla/uniffi-rs), which generates the Swift and Kotlin bindings.

| Crate | Contents |
|---|---|
| `melde-core` | Shared types: dates, subjects, lessons |
| `melde-grades` | Grade scales, weighting, rounding, "grade needed" |
| `melde-scheduling` | Timetable, A/B weeks, bell schedule |
| `melde-statistics` | Participation counts, streaks, weekly goals |
| `melde-tasks` | Homework and to-do rules, recurrence |
| `melde-ffi` | The UniFFI surface both apps bind to |
| `tools/uniffi-bindgen` | Pinned binding generator |

Both apps also take their strings from here. `l10n/Localizable.xcstrings` is the German and English string catalog.

## Use

Each app includes this repo as a git submodule at `core/`:

- [meldo-ios](https://github.com/meldo-dev/meldo-ios) runs `scripts/build-rust-ios.sh` to build an XCFramework and the Swift bindings.
- [meldo-android](https://github.com/meldo-dev/meldo-android) lets Gradle build the `.so` libraries with `cargo-ndk` and generate the Kotlin bindings.

## Develop

```bash
cargo test --workspace
```

```bash
cargo clippy --workspace --all-targets -- -D warnings
```

When you change the core, bump the submodule in both app repos so they pick it up.

## License

[MIT](LICENSE)
