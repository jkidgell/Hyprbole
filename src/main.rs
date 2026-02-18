use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use base64::{engine::general_purpose::STANDARD, Engine};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    DefaultTerminal, Frame,
};

const MAX_WALLPAPERS: usize = 100;

const BANNER: &[&str] = &[
    " _                       _           _      ",
    "| |__  _   _ _ __  _ __ | |__   ___ | | ___ ",
    "| '_ \\| | | | '_ \\| '__|| '_ \\ / _ \\| |/ _ \\",
    "| | | | |_| | |_) | |   | |_) | (_) | |  __/",
    "|_| |_|\\__, | .__/|_|   |_.__/ \\___/|_|\\___|",
    "        |___/|_|                             ",
];

struct Wallpaper {
    path: PathBuf,
    filename: String,
}

struct Monitor {
    name: String,
}

struct AppState {
    wallpaper_dir: PathBuf,
    wallpapers: Vec<Wallpaper>,
    list_state: ListState,
    monitors: Vec<Monitor>,
    active_monitor_index: usize,
    running: bool,
    last_preview_index: Option<usize>,
}

impl AppState {
    fn new(wallpaper_dir: PathBuf, wallpapers: Vec<Wallpaper>, monitors: Vec<Monitor>) -> Self {
        let mut list_state = ListState::default();
        if !wallpapers.is_empty() {
            list_state.select(Some(0));
        }
        Self {
            wallpaper_dir,
            wallpapers,
            list_state,
            monitors,
            active_monitor_index: 0,
            running: true,
            last_preview_index: None,
        }
    }

    fn rescan(&mut self) {
        let selected_filename = self
            .selected_wallpaper()
            .map(|w| w.filename.clone());

        if let Ok(wallpapers) = scan_wallpapers(&self.wallpaper_dir) {
            self.wallpapers = wallpapers;

            // Try to restore selection by filename
            let new_index = selected_filename.and_then(|name| {
                self.wallpapers.iter().position(|w| w.filename == name)
            });

            let index = match new_index {
                Some(i) => i,
                None if !self.wallpapers.is_empty() => 0,
                _ => {
                    self.list_state.select(None);
                    self.last_preview_index = None;
                    return;
                }
            };

            self.list_state.select(Some(index));
            // Force preview refresh
            self.last_preview_index = None;
        }
    }

    fn selected_wallpaper(&self) -> Option<&Wallpaper> {
        self.list_state
            .selected()
            .and_then(|i| self.wallpapers.get(i))
    }

    fn active_monitor(&self) -> Option<&Monitor> {
        self.monitors.get(self.active_monitor_index)
    }

    fn next_monitor(&mut self) {
        if !self.monitors.is_empty() {
            self.active_monitor_index =
                (self.active_monitor_index + 1).min(self.monitors.len() - 1);
        }
    }

    fn previous_monitor(&mut self) {
        self.active_monitor_index = self.active_monitor_index.saturating_sub(1);
    }
}

fn scan_wallpapers(dir: &Path) -> io::Result<Vec<Wallpaper>> {
    let mut wallpapers = Vec::new();
    scan_recursive(dir, &mut wallpapers)?;
    wallpapers.sort_by(|a, b| a.filename.cmp(&b.filename));
    wallpapers.truncate(MAX_WALLPAPERS);
    Ok(wallpapers)
}

fn scan_recursive(dir: &Path, wallpapers: &mut Vec<Wallpaper>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            scan_recursive(&path, wallpapers)?;
        } else if is_wallpaper(&path) {
            if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                wallpapers.push(Wallpaper {
                    path: path.clone(),
                    filename: filename.to_string(),
                });
            }
        }
    }
    Ok(())
}

fn is_wallpaper(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(),
        Some("jpg" | "jpeg" | "png")
    )
}

fn discover_monitors() -> io::Result<Vec<Monitor>> {
    let output = Command::new("hyprctl")
        .args(["monitors", "-j"])
        .output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "hyprctl monitors failed",
        ));
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

    let monitors = json
        .as_array()
        .unwrap_or(&vec![])
        .iter()
        .filter_map(|m| m["name"].as_str())
        .map(|name| Monitor {
            name: name.to_string(),
        })
        .collect::<Vec<_>>();

    if monitors.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no monitors found",
        ));
    }

    Ok(monitors)
}

fn set_wallpaper(monitor: &str, wallpaper: &Path) -> io::Result<()> {
    let arg = format!("{},{}", monitor, wallpaper.display());
    let output = Command::new("hyprctl")
        .args(["hyprpaper", "wallpaper", &arg])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("hyprctl hyprpaper wallpaper failed: {}", stderr.trim()),
        ));
    }

    Ok(())
}

fn cell_size() -> Option<(f64, f64)> {
    let ws = crossterm::terminal::window_size().ok()?;
    if ws.columns == 0 || ws.rows == 0 || ws.width == 0 || ws.height == 0 {
        return None;
    }
    Some((
        ws.width as f64 / ws.columns as f64,
        ws.height as f64 / ws.rows as f64,
    ))
}

fn kitty_clear() {
    let mut out = io::stdout().lock();
    // q=2 suppresses responses, d=A deletes all images and frees memory
    let _ = out.write_all(b"\x1b_Ga=d,d=A,q=2\x1b\\");
    let _ = out.flush();
}

fn kitty_display(path: &Path, area: Rect) {
    // Inner area: account for block border
    let col = area.x + 1;
    let row = area.y + 1;
    let cols = area.width.saturating_sub(2) as u32;
    let rows = area.height.saturating_sub(2) as u32;

    if cols == 0 || rows == 0 {
        return;
    }

    // Decode image
    let img = match image::open(path) {
        Ok(img) => img,
        Err(_) => return,
    };

    // Calculate target pixel size from cell dimensions
    let (cw, ch) = cell_size().unwrap_or((8.0, 16.0));
    let target_w = (cols as f64 * cw) as u32;
    let target_h = (rows as f64 * ch) as u32;

    if target_w == 0 || target_h == 0 {
        return;
    }

    // Resize preserving aspect ratio
    let resized = img.thumbnail(target_w, target_h);
    let rgb = resized.to_rgb8();
    let (w, h) = rgb.dimensions();
    let raw = rgb.as_raw();

    // Base64 encode and chunk into 4096-byte pieces (per Kitty protocol spec)
    let b64 = STANDARD.encode(raw);
    let chunks: Vec<&str> = b64.as_bytes().chunks(4096).map(|c| {
        // SAFETY: base64 output is always valid ASCII/UTF-8
        unsafe { std::str::from_utf8_unchecked(c) }
    }).collect();

    let mut out = io::stdout().lock();

    // Move cursor to preview area (ANSI is 1-indexed)
    let _ = write!(out, "\x1b[{};{}H", row + 1, col + 1);

    // Send first chunk with all control parameters
    let more = if chunks.len() > 1 { 1 } else { 0 };
    if let Some(first) = chunks.first() {
        let _ = write!(
            out,
            "\x1b_Gq=2,a=T,z=-1,C=1,f=24,s={},v={},m={};{}\x1b\\",
            w, h, more, first
        );
    }

    // Send continuation chunks
    for (i, chunk) in chunks.iter().enumerate().skip(1) {
        let more = if i < chunks.len() - 1 { 1 } else { 0 };
        let _ = write!(out, "\x1b_Gm={};{}\x1b\\", more, chunk);
    }

    let _ = out.flush();
}

fn update_preview(state: &mut AppState, preview_area: Rect) {
    let current = state.list_state.selected();
    if current == state.last_preview_index {
        return;
    }

    kitty_clear();
    state.last_preview_index = current;

    if let Some(wp) = current.and_then(|i| state.wallpapers.get(i)) {
        kitty_display(&wp.path, preview_area);
    }
}

fn ui(frame: &mut Frame, state: &mut AppState) -> Rect {
    let [banner_area, main_area, status_area] = Layout::vertical([
        Constraint::Length(BANNER.len() as u16),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let split = 24usize;
    let banner_lines: Vec<Line> = BANNER
        .iter()
        .map(|&s| {
            let (hypr, bole) = if s.len() >= split {
                (&s[..split], &s[split..])
            } else {
                (s, "")
            };
            Line::from(vec![
                Span::styled(hypr, Style::default().fg(Color::Rgb(255, 0, 182))),
                Span::styled(bole, Style::default().fg(Color::Cyan)),
            ])
        })
        .collect();
    frame.render_widget(Paragraph::new(banner_lines), banner_area);

    let [list_area, preview_area] = Layout::horizontal([
        Constraint::Percentage(40),
        Constraint::Percentage(60),
    ])
    .areas(main_area);

    // Wallpaper list
    let monitor_name = state
        .active_monitor()
        .map(|m| m.name.as_str())
        .unwrap_or("none");

    let title = format!(" Wallpapers [{}] ", monitor_name);
    let items: Vec<ListItem> = state
        .wallpapers
        .iter()
        .map(|w| ListItem::new(w.filename.as_str()))
        .collect();

    let list = List::new(items)
        .block(Block::default()
            .title(Line::from(title).style(Style::default().fg(Color::Rgb(255, 0, 182))))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)))
        .highlight_symbol("> ")
        .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

    frame.render_stateful_widget(list, list_area, &mut state.list_state);

    // Preview pane (empty block — Kitty draws the image after render)
    let preview_block = Block::default()
        .title(Line::from(" Preview ").style(Style::default().fg(Color::Rgb(255, 0, 182))))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(preview_block, preview_area);

    // Status bar
    let status = Line::from(vec![
        Span::styled("Enter", Style::default().fg(Color::Rgb(255, 0, 182))),
        Span::raw(": Apply  "),
        Span::styled("r", Style::default().fg(Color::Rgb(255, 0, 182))),
        Span::raw(": Rescan  "),
        Span::styled("j/k ↑↓", Style::default().fg(Color::Rgb(255, 0, 182))),
        Span::raw(": Navigate  "),
        Span::styled("←→", Style::default().fg(Color::Rgb(255, 0, 182))),
        Span::raw(": Monitor  "),
        Span::styled("q", Style::default().fg(Color::Rgb(255, 0, 182))),
        Span::raw(": Quit"),
    ]);

    frame.render_widget(Paragraph::new(status), status_area);

    preview_area
}

fn handle_key(state: &mut AppState, code: KeyCode) {
    match code {
        KeyCode::Char('q') => state.running = false,
        KeyCode::Char('j') | KeyCode::Down => state.list_state.select_next(),
        KeyCode::Char('k') | KeyCode::Up => state.list_state.select_previous(),
        KeyCode::Left => state.previous_monitor(),
        KeyCode::Right => state.next_monitor(),
        KeyCode::Char('r') => state.rescan(),
        KeyCode::Enter => {
            if let (Some(wp), Some(mon)) = (state.selected_wallpaper(), state.active_monitor()) {
                let _ = set_wallpaper(&mon.name, &wp.path);
            }
        }
        _ => {}
    }
}

fn run(terminal: &mut DefaultTerminal, state: &mut AppState) -> io::Result<()> {
    while state.running {
        let mut preview_area = Rect::default();
        terminal.draw(|frame| {
            preview_area = ui(frame, state);
        })?;

        update_preview(state, preview_area);

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                handle_key(state, key.code);
            }
        }
    }
    kitty_clear();
    Ok(())
}

fn main() -> io::Result<()> {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| String::from("."));

    let wallpaper_dir = PathBuf::from(&dir);
    let wallpapers = scan_wallpapers(&wallpaper_dir)?;
    let monitors = discover_monitors()?;
    let mut state = AppState::new(wallpaper_dir, wallpapers, monitors);

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut state);
    ratatui::restore();
    result
}
