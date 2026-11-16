# noteking-tui

<p align="center">
  <strong>A fast, keyboard-driven terminal user interface (TUI) for the <a href="https://github.com/leouofa/noteking">NoteKing</a> Markdown note studio and developer workspace.</strong>
</p>

<p align="center">
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-2021_Edition-dea584?style=flat-square&logo=rust&logoColor=white" alt="Rust 2021"></a>
  <a href="https://ratatui.rs"><img src="https://img.shields.io/badge/Ratatui-0.28-111111?style=flat-square" alt="Ratatui"></a>
  <a href="https://github.com/crossterm-rs/crossterm"><img src="https://img.shields.io/badge/Crossterm-0.28-blue?style=flat-square" alt="Crossterm"></a>
  <img src="https://img.shields.io/badge/Architecture-Local--First-22c55e?style=flat-square" alt="Local-First Architecture">
  <a href="https://github.com/leouofa/noteking"><img src="https://img.shields.io/badge/NoteKing-Rails_8-D30001?style=flat-square&logo=rubyonrails&logoColor=white" alt="NoteKing Engine"></a>
  <a href="https://github.com/leouofa/noteking-mcp"><img src="https://img.shields.io/badge/MCP_Server-noteking--mcp-6366f1?style=flat-square&logo=github&logoColor=white" alt="NoteKing MCP Server"></a>
  <a href="https://opensource.org/licenses/MIT"><img src="https://img.shields.io/badge/License-MIT-blue?style=flat-square" alt="License: MIT"></a>
</p>

---

A fast, keyboard-driven terminal user interface (TUI) for the [NoteKing](https://github.com/leouofa/noteking) Markdown note-taking studio and developer workbench built with [Ratatui](https://ratatui.rs) and [Crossterm](https://github.com/crossterm-rs/crossterm).

## Features

- **Host URL Resolution & Host Switcher**:
  - **Environment default**: When `NOTEKING_REMOTE_URL` is set in the shell (e.g. `export NOTEKING_REMOTE_URL="http://100.96.219.81:3009"`), running `cargo run` or `noteking-tui` connects directly to that URL.
  - **Fallback**: If `NOTEKING_REMOTE_URL` is not set, defaults to `http://localhost:3000`.
  - **CLI overrides**: `--url <URL>`, `--host <URL>`, `--remote`, and `--local` (`http://localhost:3000`) always take precedence over the environment.
  - **Locking behavior**:
    - If `NOTEKING_REMOTE_URL` is set, host switching is locked by default so the TUI stays pinned to your configured instance.
    - Pass `--allow-switch` to re-enable the Host Switcher dialog while using `NOTEKING_REMOTE_URL`.
    - If `NOTEKING_REMOTE_URL` is not set, Host Switcher (`H` or `u`) is enabled automatically.
- **3-Pane Master–Detail Layout (Amber/Gold Theme)**:
  - **Header**: Live host URL badge, workspace statistics (Notes, Pinned, Standalone, Projects, Attachments), active Markdown view mode, and search filter badge.
  - **Pane 1 `[1] Scopes & Projects`**: All Notes, ★ Pinned, Standalone scratchpads, and all Projects with note counts.
  - **Pane 2 `[2] Notes`**: Filtered list showing pin markers (`★`), note titles, optional checklist progress badges (`[done/total]`), attachment counters (`📎`), project badges, and updated dates.
  - **Pane 3 `[3] Note Detail [Preview | Raw]`**: Note metadata header (project, word/line counts, updated timestamp, attachment counters) and rich Markdown body.
- **Dual Markdown View Modes & Smart Checklists**:
  - `v` / `m` toggles between **Styled Preview Mode** (GFM headings, fenced code blocks, blockquotes, tables, horizontal rules, bullet lists, word wrap) and **Raw Markdown Mode** (with line numbers).
  - If a note contains `- [ ]` / `- [x]` checklist items, `j` / `k` in Preview mode navigates directly between tasks and `Space` / `Enter` toggles completion state back to the server.
  - `J` / `K`, `PageDown` / `PageUp`, and `g` / `G` scroll the full document smoothly.
- **Full CRUD, Quick Append & External `$EDITOR`**:
  - `a`: **Quick Append to Note** modal — append a Markdown paragraph, log entry, or checklist item (`- [ ]`) under an optional `## Section` header without leaving the TUI.
  - `n` / `e`: Create or edit Projects (in Pane 1) or Notes (in Pane 2/3).
  - `d`: Delete Project (preserving notes as standalone), Note, or Attachment with confirmation dialog.
  - `p`: Toggle pin status on selected Note.
  - `K` / `J`: Reorder pinned Notes when viewing the Pinned scope.
  - `E`: Open full Markdown body in external `$EDITOR` (e.g. `nvim`, `vim`, `nano`). Suspends TUI cleanly and saves back to NoteKing on exit.
- **Attachments & Zero-Latency Clipboard Copy**:
  - `A`: Open Context Attachment Manager modal to view attachments, upload new files by path (`u`), download files to disk (`s`), copy attachment URLs (`y`), or purge files (`d`).
  - `c` / `Y`: Copy entire Markdown body to system clipboard (`arboard`).
  - `y`: Yank active checklist item text (or note title) to system clipboard.
- **Live Search**:
  - `/`: Real-time full-text search across note titles and bodies. Press `Esc` to clear.

## Installation & Running

> [!NOTE]
> `noteking-tui` connects to a running [NoteKing](https://github.com/leouofa/noteking) backend instance (defaults to `NOTEKING_REMOTE_URL` or `http://localhost:3000`).

### Build from Source

```bash
# Build
cargo build --release

# Run (defaults to NOTEKING_REMOTE_URL if set, otherwise http://localhost:3000)
cargo run

# Force local host even if NOTEKING_REMOTE_URL is set
cargo run -- --local

# Specify any target host explicitly
cargo run -- --url http://100.96.219.81:3009

# Allow in-app Host Switcher even when NOTEKING_REMOTE_URL is set
cargo run -- --allow-switch
```

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `1`, `2`, `3` | Direct focus to Pane 1 (Sidebar), 2 (Notes), 3 (Detail) |
| `Tab` / `BackTab` | Cycle focus forward / backward between panes |
| `h` / `l` | Move focus left / right |
| `j` / `k` | Move selection down / up in focused pane |
| `Enter` | Enter pane 2 from sidebar, or pane 3 from notes list |
| `v` / `m` | Toggle between **Preview** (styled GFM) and **Raw** (line-numbered) Markdown modes |
| `Space` / `Enter` | Toggle active checklist item `[ ]` ⇄ `[x]` (in Pane 3 when tasks exist) |
| `a` | Quick-append Markdown content or task (`- [ ]`) to current note |
| `E` | Open note Markdown body in `$EDITOR` |
| `J` / `K` | Scroll detail document down / up |
| `g` / `G` | Jump to top / bottom of detail document |
| `c` / `Y` | Copy full Markdown body to system clipboard |
| `y` | Yank active task text or note title to system clipboard |
| `H` / `u` | Open Host Switcher dialog (when enabled or via `--allow-switch`) |
| `/` | Live search & filter notes (`Esc` to clear) |
| `n` | New Project (in pane 1) or Note (in pane 2/3) |
| `e` | Edit Project or Note metadata |
| `d` | Delete with confirmation |
| `p` | Toggle pin on selected Note |
| `K` / `J` | Reorder pinned notes up / down (in Pinned scope) |
| `A` | Manage note attachments |
| `r` | Refresh data from server |
| `?` | Show help modal |
| `q` | Quit application |
