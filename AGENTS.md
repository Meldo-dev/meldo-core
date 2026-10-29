# AGENTS.md

Guide for AI coding agents working in meldo-core, the Rust core of Meldo.

## What Meldo is

Meldo is a free, open source school planner for students in Germany. Its core feature counts how often a student raises their hand in class (Meldungen). It also has a timetable, homework, exams, grades, a calendar and widgets.

- Apps for iOS, iPadOS and Android. All three share one Rust core (meldo-core).
- No backend and no account. Data stays on the device.
- Optional integrations: Microsoft Teams assignments, and Google Drive backup and sync. Sync data is encrypted on the device before upload (AES-256-GCM, key from PBKDF2-SHA256) and stored in the `drive.appdata` scope.
- German is the default language, English is the second one.
- Website: meldo.dev.

Repos in the Meldo-dev org: `meldo-core`, `meldo-ios`, `meldo-android` (public), `meldo-site`, `meldo-media` (private), `.github` (org profile).

## How this repo works

meldo-core is the Rust workspace both apps share. Anything iOS and Android must compute the same way lives here.

| Path | Contents |
|---|---|
| `crates/melde-core` | Shared types: dates, subjects, lessons |
| `crates/melde-grades` | Grade scales, weighting, rounding, "grade needed" |
| `crates/melde-scheduling` | Timetable, A/B weeks, bell schedule |
| `crates/melde-statistics` | Participation counts, streaks, weekly goals |
| `crates/melde-tasks` | Homework and to-do rules, recurrence |
| `crates/melde-ffi` | The UniFFI surface both apps bind to |
| `tools/uniffi-bindgen` | Pinned binding generator |
| `l10n/Localizable.xcstrings` | The German and English string catalog for both apps |

Data flow:

- The apps include this repo as a git submodule at `core/`.
- meldo-ios runs `scripts/build-rust-ios.sh`, which builds an XCFramework and the Swift bindings.
- meldo-android lets Gradle build the `.so` libraries with `cargo-ndk` and generate the Kotlin bindings with `tools/uniffi-bindgen`.
- Both apps read `l10n/Localizable.xcstrings`. iOS uses it as its String Catalog; Android merges it into a JSON asset at build time.

Rules for the code:

- `unsafe_code` is forbidden in the workspace.
- Release builds keep `panic = "unwind"`, so UniFFI turns panics into errors instead of aborting the app. Prefer returning errors anyway; `unwrap_used` is a warning.
- Calendar math uses `chrono` with `std` only. The apps pass local wall clock values in; the core reads no clock or timezone database.
- A change to the `melde-ffi` surface changes the generated Swift and Kotlin code. Update both apps in the same round.
- Keys in the string catalog are the German text. Add the English translation for every new key.

## Build, test and release

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
```

CI (`.github/workflows/core.yml`) runs these three on Ubuntu and Windows. `hygiene.yml` runs on every push.

There are no releases here. The apps pick up a change when their `core/` submodule is bumped.

## How to commit

- Commit under your own name and GitHub no-reply email, so the history shows who made each change. Never commit as an AI tool or under someone else's name, and don't change another person's git config.
- If AI helped with a commit, end the message with the trailer `Assisted-by: AI`. No `Co-Authored-By` lines for AI tools and no "Generated with" footers.
- Short imperative commit messages in plain English that say why, for example "Keep the timetable grid within one day".
- Core changes: commit and push meldo-core first, then bump the `core/` submodule in both meldo-ios and meldo-android.
- Every user-facing string goes in `meldo-core/l10n/Localizable.xcstrings`, German first, with an English translation.
- Don't claim a build or test passed unless you ran it and saw the output.

## Allowed and not allowed

- Never put secrets in the repo, code, logs, commits or chat: OAuth client IDs (Google iOS, Google Android, Microsoft Teams), keystore passwords, API keys, tokens. Say where they come from instead: the gitignored `Config/Secrets.xcconfig` on iOS, `~/.gradle/gradle.properties` on Android, GitHub secrets in CI.
- No mention of AI tools, assistant names, private notes or decision record numbers in code, comments or commits. The `hygiene` workflow in the public repos fails on this.
- Participation counts are not grades. Grade logic goes through the scale type (1 to 6, lower is better; 0 to 15, higher is better). Grading rules are configuration, not constants.
- The sync format must stay identical on iOS and Android.
- No new dependency without a stated reason. No mock data shown as real.
- Don't push to `main`, `beta` or `alpha`, and don't create tags or releases, unless the owner asked for it. Work on a branch and open a pull request.
- `beta` and `alpha` differ from `main` on purpose. Don't merge `main` into them, and don't force-push.
- Don't touch the owner's devices or real data.
- The WebUntis import is not part of the apps for now. Don't add it back or reintroduce code for it.
- Site: no third-party requests, cookies or trackers. Keep the `/de/` and `/en/` URLs and the redirects. Legal pages: German is authoritative, English matches it.
