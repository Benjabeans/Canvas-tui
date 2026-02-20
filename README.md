# canvas-tui

A terminal UI for Canvas LMS. Browse your courses, assignments, calendar, and announcements without leaving the terminal.

![canvas-tui screenshot](https://raw.githubusercontent.com/placeholder/canvas-tui/main/screenshot.png)

---

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (1.70 or later)
- A Canvas LMS account with API access
- A Canvas API token (see below)

---

## Installation

### 1. Clone the repository

```bash
git clone https://github.com/your-username/canvas-tui.git
cd canvas-tui
```

### 2. Build

```bash
cargo build --release
```

The binary will be at `target/release/canvas-tui`. You can copy it anywhere on your `$PATH`:

```bash
cp target/release/canvas-tui ~/.local/bin/
```

---

## Configuration

### Get a Canvas API token

1. Log in to your Canvas instance
2. Go to **Account → Settings**
3. Scroll to **Approved Integrations** and click **+ New Access Token**
4. Give it a name and click **Generate Token**
5. Copy the token — you won't be able to see it again

### Create the config file

Run the init command to generate a template:

```bash
canvas-tui --init
```

This creates `~/.config/canvas-tui/config.toml`. Open it and fill in your details:

```toml
canvas_url = "https://your-school.instructure.com"
api_token = "your-api-token-here"
```

Replace `your-school.instructure.com` with your institution's Canvas domain (e.g. `canvas.university.edu`).

### Alternative: environment variables

Instead of a config file you can set environment variables:

```bash
export CANVAS_URL="https://your-school.instructure.com"
export CANVAS_API_TOKEN="your-api-token-here"
canvas-tui
```

---

## Usage

```bash
canvas-tui
```

The app loads cached data instantly on startup and syncs with Canvas in the background.

### Tabs

| Key | Tab |
|-----|-----|
| `1` | Dashboard — overview and upcoming assignments |
| `2` | Courses |
| `3` | Assignments |
| `4` | Calendar |
| `5` | Announcements |
| `Tab` / `Shift+Tab` | Cycle tabs forward / backward |

### Navigation

| Key | Action |
|-----|--------|
| `j` / `k` or `↓` / `↑` | Move down / up |
| `g` / `G` | Jump to top / bottom |
| `t` | Jump to today (Calendar and Assignments tabs) |

### Assignments tab

| Key | Action |
|-----|--------|
| `s` | Cycle sort: Due ↑ → Due ↓ → Course → Status |
| `f` | Open course filter popup |

**Course filter popup:**

| Key | Action |
|-----|--------|
| `j` / `k` | Navigate courses |
| `Space` | Toggle a course on/off |
| `Enter` / `Esc` / `f` | Close popup |

When no courses are toggled, all courses are shown. Toggling any course switches to an explicit include list.

### General

| Key | Action |
|-----|--------|
| `r` | Refresh data from Canvas |
| `q` / `Ctrl+C` | Quit |

---

## Data & caching

Fetched data is cached locally so the app is usable offline and starts instantly. Cache is stored at:

```
~/.cache/canvas-tui/cache.json   (Linux/macOS)
```

Press `r` to force a fresh sync at any time.
