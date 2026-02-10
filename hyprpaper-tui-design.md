# Hyprpaper TUI Wallpaper Selector — Design Document

## Overview

This project is a **keyboard-driven TUI wallpaper selector** for **Hyprland** using **hyprpaper**.

- Platform: **Arch Linux only**
- Display server: **Wayland (Hyprland)**
- Language: **Rust**
- TUI library: **Ratatui + Crossterm**
- Preview rendering: **Kitty graphics protocol (external preview, like Yazi)**

The application allows the user to browse wallpapers from a single directory, preview them, and apply them per-monitor using hyprpaper.

This is **not** intended to be portable or cross-platform.

---

## Goals

- Fast, minimal, keyboard-only workflow
- Vim-style navigation
- Instant wallpaper switching (via hyprpaper preload)
- Per-monitor wallpaper selection
- Simple architecture with future extensibility

---

## Non-Goals

- No GUI (GTK/Qt/etc.)
- No thumbnail database
- No async runtime
- No external wallpaper managers
- No scripting languages
- No Windows/macOS support

---

## Environment Assumptions

- Arch Linux
- Hyprland compositor
- hyprpaper already installed and running
- Existing `hyprpaper.conf` is present and respected
- Terminal: **Kitty** (required for image previews)

---

## Wallpaper Source

- Single wallpaper directory
- Recursive scanning enabled
- File types:
  - `.jpg`
  - `.png`
- Maximum of **100 wallpapers**
- Directory can be rescanned on demand to detect new files

---

## Application Architecture

```
Filesystem
   ↓
Wallpaper Index (Vec<Wallpaper>)
   ↓
Application State
   ↓
Ratatui UI
   ↓
Hyprpaper Adapter (hyprctl)
```

The application is structured around a **single mutable AppState** updated by keyboard events.

---

## Core Data Models (Conceptual)

### Wallpaper
- path: PathBuf
- filename: String

### Monitor
- name: String (e.g. HDMI-A-1)

### AppState
- wallpapers: Vec<Wallpaper>
- selected_index: usize
- monitors: Vec<Monitor>
- active_monitor_index: usize
- last_scan_time

---

## UI Layout

```
┌────────────────────────────────────────────┐
│ Wallpapers                                 │
│ ───────────────────────────────────────── │
│ > forest.png                               │
│   city.jpg                                 │
│   desert.png                               │
│                                            │
│ Monitor: HDMI-A-1                          │
├────────────────────────────────────────────┤
│ Enter: Apply  r: Rescan  q: Quit           │
│ j/k or ↑↓ to navigate, ← → to change monitor│
└────────────────────────────────────────────┘
```

---

## Keyboard Controls

| Key        | Action                         |
|------------|--------------------------------|
| j / ↓      | Move selection down            |
| k / ↑      | Move selection up              |
| ← / →      | Change active monitor          |
| Enter      | Apply wallpaper immediately    |
| r          | Rescan wallpaper directory     |
| q          | Quit application               |

---

## Preview Strategy (Kitty)

- Image preview is displayed using **Kitty graphics protocol**
- Preview updates when selection changes
- Preview rendering is **not part of Ratatui**
- The TUI only triggers preview updates via external commands
- Behavior should be similar to Yazi’s image preview

---

## Hyprpaper Integration

### Monitor Discovery
- Use:
  ```
  hyprctl monitors -j
  ```

### Preloading
- All wallpapers are preloaded at startup
- On rescan, only newly discovered wallpapers are preloaded

### Applying Wallpaper
```
hyprctl hyprpaper wallpaper "<monitor>,<path>"
```

---

## Configuration Strategy

- Single compiled binary
- Optional TOML configuration (future use)
- Default config path:
  ```
  ~/.config/hyprpaper-tui/config.toml
  ```

---

## MVP Scope

Included:
- Recursive directory scan
- Wallpaper list UI
- Kitty preview
- Per-monitor wallpaper selection
- hyprpaper preload + apply
- Rescan support

Excluded (for now):
- Tagging
- Random rotation
- Timers
- Profiles
- Persistent history

---

## Development Constraints

- Stable Rust only
- Ratatui idioms must be respected
- Code must compile at every step
- Minimal dependencies

---

## Success Criteria

- Application starts instantly
- Navigation is smooth with ≤100 wallpapers
- Wallpaper changes are instant
- No flicker
- Clean exit restores terminal state
