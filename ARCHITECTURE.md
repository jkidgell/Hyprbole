# Hyprbole — Architecture Reference

## What It Is

Keyboard-driven TUI wallpaper selector for Hyprland. Scans a directory for images,
shows a list with Kitty image preview, applies wallpapers per-monitor via hyprpaper.

## Constraints

- Arch Linux + Hyprland + Kitty only
- Stable Rust, Ratatui 0.29 + Crossterm 0.28
- No async runtime — single-threaded event loop
- Single file: `src/main.rs` (~395 lines)

## Dependencies (Cargo.toml)

| Crate       | Purpose                              |
|-------------|--------------------------------------|
| ratatui     | TUI framework (layout, widgets)      |
| crossterm   | Terminal backend (input, raw mode)    |
| serde_json  | Parse `hyprctl monitors -j` output   |
| base64      | Encode pixel data for Kitty protocol |
| image       | Decode/resize JPG/PNG to raw RGB     |

## Data Flow

```
CLI arg (directory path)
  → scan_wallpapers()      recursive dir scan, sorted, max 100
  → discover_monitors()    hyprctl monitors -j → Vec<Monitor>
  → AppState::new()        initializes selection, list state
  → run() event loop:
      terminal.draw()      Ratatui renders list + empty preview block
      update_preview()     Kitty graphics protocol draws image
      event::read()        blocks for keyboard input
      handle_key()         updates state
```

## Key Structs

### Wallpaper (line 18)
- `path: PathBuf` — absolute path to image file
- `filename: String` — display name in list

### Monitor (line 23)
- `name: String` — Hyprland monitor name (e.g. "DP-1")

### AppState (line 27)
- `wallpaper_dir: PathBuf` — scan directory, kept for rescan
- `wallpapers: Vec<Wallpaper>` — current wallpaper list
- `list_state: ListState` — Ratatui's built-in selection + scroll state
- `monitors: Vec<Monitor>` — discovered monitors
- `active_monitor_index: usize` — which monitor to apply wallpapers to
- `running: bool` — event loop control
- `last_preview_index: Option<usize>` — tracks preview to avoid redundant redraws

## Key Functions

| Function              | Line | Purpose                                              |
|-----------------------|------|------------------------------------------------------|
| `scan_wallpapers()`   | 105  | Entry point: recursive scan, sort, truncate to 100   |
| `scan_recursive()`    | 113  | Walks subdirectories collecting .jpg/.jpeg/.png       |
| `is_wallpaper()`      | 132  | Extension check (case-insensitive)                   |
| `discover_monitors()` | 139  | Runs `hyprctl monitors -j`, parses JSON for names    |
| `set_wallpaper()`     | 174  | Runs `hyprctl hyprpaper wallpaper "<mon>,<path>"`    |
| `cell_size()`         | 191  | Queries terminal pixel/cell dimensions via crossterm  |
| `kitty_clear()`       | 202  | Sends Kitty delete-all-images escape sequence         |
| `kitty_display()`     | 209  | Decodes image, resizes, sends as chunked raw RGB      |
| `update_preview()`    | 272  | Only redraws preview when selection actually changes   |
| `ui()`                | 286  | Ratatui layout: list (40%) | preview (60%) | status  |
| `handle_key()`        | 344  | Key dispatch: j/k/arrows/Enter/r/q                   |
| `run()`               | 361  | Main event loop: draw → preview → input → repeat     |

## Kitty Graphics Protocol (lines 202-269)

Uses Yazi's KgpOld approach — NOT file paths (`t=f`), but direct raw pixel data:

1. `image::open()` decodes JPG/PNG
2. `thumbnail()` resizes to fit preview area (cell size × cell count)
3. Convert to RGB8, base64 encode raw bytes
4. Chunk into 4096-byte pieces per Kitty spec
5. First chunk: `\x1b_Gq=2,a=T,z=-1,C=1,f=24,s={w},v={h},m={more};{data}\x1b\\`
6. Continuation chunks: `\x1b_Gm={more};{data}\x1b\\`
7. Clear: `\x1b_Ga=d,d=A,q=2\x1b\\`

Key parameters: `q=2` (suppress responses), `C=1` (don't move cursor),
`z=-1` (below text), `f=24` (RGB), `s`/`v` (pixel dimensions).

## UI Layout

```
┌─ Wallpapers [DP-1] ──┬─ Preview ─────────────┐
│ > forest.png          │                       │
│   city.jpg            │   [Kitty image here]  │
│   desert.png          │                       │
│                       │                       │
├───────────────────────┴───────────────────────┤
│ Enter: Apply  r: Rescan  j/k: Nav  ←→: Mon   │
└───────────────────────────────────────────────┘
```

- Left pane: 40% — wallpaper list with `ListState` selection
- Right pane: 60% — empty Ratatui block, Kitty draws image on top after render
- Bottom: 1-line status bar with key hints

## Keybindings

| Key       | Action                          | Handler                    |
|-----------|---------------------------------|----------------------------|
| j / ↓     | Select next wallpaper           | `list_state.select_next()` |
| k / ↑     | Select previous wallpaper       | `list_state.select_previous()` |
| ← / →     | Switch active monitor           | `previous/next_monitor()`  |
| Enter     | Apply wallpaper to monitor      | `set_wallpaper()`          |
| r         | Rescan directory                | `AppState::rescan()`       |
| q         | Quit                            | `state.running = false`    |

## Hyprpaper Integration

- Monitor discovery: `hyprctl monitors -j` → parse JSON array for `"name"` fields
- Apply wallpaper: `hyprctl hyprpaper wallpaper "<monitor>,<absolute_path>"`
- No separate preload command needed — `wallpaper` handles both

## Installed Locations

| What                | Path                                                  |
|---------------------|-------------------------------------------------------|
| Binary              | `~/.cargo/bin/hyprbole`                               |
| Source              | `~/Projects/Hyprbole/src/main.rs`                     |
| Hyprland keybind    | `~/.config/hypr/hyprland.conf` (SUPER+W)              |
| Desktop entry       | `~/.local/share/applications/hyprbole.desktop`        |
| Wallpaper directory | `~/Pictures/Wallpapers` (passed as CLI arg)           |

## Rebuilding

```sh
cd ~/Projects/Hyprbole
cargo install --path .
```

## Common Modifications

**Add file types**: Edit `is_wallpaper()` line 132 — add extensions to the `matches!` arm.

**Change max wallpapers**: Edit `MAX_WALLPAPERS` constant, line 16.

**Change layout split**: Edit `Constraint::Percentage` values in `ui()` line 293-294.

**Change keybindings**: Edit `handle_key()` match arms, line 344.

**Change wallpaper command**: Edit `set_wallpaper()` line 174 — modify the `hyprctl` args.

**Change colors/styles**: Edit `ui()` — `highlight_style` at line 315, status bar spans at line 326.

**Add window rules in Hyprland**: The Kitty window launches with `--class hyprbole`, so:
```
windowrulev2 = float, class:^(hyprbole)$
windowrulev2 = size 60% 70%, class:^(hyprbole)$
windowrulev2 = center, class:^(hyprbole)$
```
