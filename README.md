# Fjord

Local-first project management for Linux. Projects, kanban boards, files and notes in one
SQLite database on your machine. No account, no cloud. Use it with the mouse, or entirely
from the keyboard (LazyVim-style leader key and which-key popup). AI agents can work in the
same projects through the `fjord` CLI.

> Status: early preview (v0.1, not yet published).

## Run it

Requirements: Rust, Node.js, `webkit2gtk-4.1` (Arch: `pacman -S webkit2gtk-4.1`).

```sh
npm --prefix ui install
npx --prefix ui tauri dev          # development, hot reload
npx --prefix ui tauri build        # release build + AppImage/.deb in target/release/bundle
./target/release/fjord-app         # run the release binary
```

CLI:

```sh
cargo run -p fjord-cli -- project add "Bokost" --icon 💸
cargo run -p fjord-cli -- task add bokost "Fix push notifications" -p 3 --due 2026-10-10
cargo run -p fjord-cli -- project show bokost
cargo run -p fjord-cli -- --json search push     # JSON for scripts and agents
```

Data lives in `~/.local/share/fjord` (override with `FJORD_DATA_DIR`). Changes are logged
with an actor (`$FJORD_ACTOR`, else `$USER`); agents should use `--actor claude`.

## Keyboard

Press `?` in the app for every shortcut. Highlights: `Ctrl+K` palette, `Space` leader
(`Space p` switch project, `Space t` new task, `Space f` search), `h j k l` to move on the
board, `H`/`L` to move a task between columns, `x` done, `d d` archive.

Override any binding in `~/.config/fjord/keymap.json`:

```json
{
  "leader": "space",
  "bindings": { "task.archive": "D", "search.open": "<leader>/" }
}
```

Action ids are listed in `ui/src/keymap.ts`.

## Layout

```
crates/fjord-core   Rust library: schema, projects, tasks, files, notes, search, activity
crates/fjord-cli    `fjord` command-line tool
src-tauri           Tauri desktop shell (commands over fjord-core)
ui                  React + TypeScript interface
```

## License

MIT OR Apache-2.0
