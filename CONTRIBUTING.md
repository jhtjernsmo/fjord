# Contributing to Fjord

Thanks for helping out! Bug reports, ideas, translations and pull requests are all welcome.

## Getting started

```sh
npm --prefix ui install
npx --prefix ui tauri dev        # app with hot reload
```

Use a throwaway database while developing so you don't touch your real projects:

```sh
FJORD_DATA_DIR=/tmp/fjord-dev npx --prefix ui tauri dev
```

## Before you open a pull request

```sh
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm --prefix ui run typecheck
npm --prefix ui test
```

CI runs the same checks on every pull request, on Linux, Windows and macOS. `main` only changes
through pull requests whose checks pass.

## Tests

- **New features come with tests**, and bug fixes with a test that would have caught the bug.
- Rust logic is tested next to the code (`#[cfg(test)]`); the CLI and MCP server have end-to-end
  tests in `crates/fjord-cli/tests`; UI logic is tested with Vitest in `ui/tests`.
- Tests that need something CI doesn't have (the system credential store) are marked `#[ignore]`;
  run them with `cargo test -- --ignored`.

## Guidelines

- **Business logic lives in `crates/fjord-core`** so the GUI, CLI and MCP server behave the same.
  Add tests next to the code (`#[cfg(test)]`).
- **Agents never hard-delete.** The MCP server only archives, and the app asks before deleting.
  Every change goes through `Store::log` so it shows up in the activity log.
- **No emoji in the interface.** Use Lucide icons or the monospace glyphs in `ui/src/components/Icons.tsx`.
- **Every user-facing string goes through i18n** (`ui/src/i18n.ts`). English is the source of truth.
- **Keyboard and mouse are equal.** A new feature needs a mouse path, and a keyboard path where it
  makes sense (add the action to `ui/src/keymap.ts`).
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org) (`feat:`, `fix:`, `docs:` …).

## Adding a language

1. Add the locale to `LOCALES` in `ui/src/i18n.ts`.
2. Copy the `no` dictionary, translate it, and register it in `DICTIONARIES`. Missing keys fall back to English.
3. Optionally add default column names in `status_names` in `crates/fjord-core/src/projects.rs`.

## Releases

Each release is listed in [CHANGELOG.md](CHANGELOG.md). Releases are built by CI from a version
tag and signed.

## Reporting bugs

Please include your distribution, how you installed Fjord, the steps to reproduce, and what you
expected to happen. Never attach your `fjord.db` if it contains private data. Security problems go
through the private process in [SECURITY.md](SECURITY.md).
