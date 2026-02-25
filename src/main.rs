mod api;
mod cache;
mod config;
mod models;
mod tui;

use anyhow::Result;
use crossterm::{
    event::{Event, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Write};
use std::time::Duration;

use api::CanvasClient;
use config::Config;
use tui::{App, SubmissionState};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--init") {
        let path = Config::generate_default()?;
        println!("Generated config file at: {}", path.display());
        println!("Edit it with your Canvas URL and API token, then run canvas-tui.");
        return Ok(());
    }

    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("canvas-tui — A terminal UI for Canvas LMS");
        println!();
        println!("USAGE:");
        println!("  canvas-tui           Start the TUI");
        println!("  canvas-tui --init    Generate a default config file");
        println!();
        println!("CONFIG:");
        println!("  File: ~/.config/canvas-tui/config.toml");
        println!("  Or set env vars: CANVAS_URL and CANVAS_API_TOKEN");
        println!();
        println!("KEYBINDINGS:");
        println!("  Tab / Shift+Tab   Switch tabs");
        println!("  1-5               Jump to tab");
        println!("  j / k / Up / Down Navigate lists");
        println!("  g / G             Jump to top / bottom");
        println!("  q / Ctrl+C        Quit");
        return Ok(());
    }

    // Try to load config; if it fails we may still have cache to show.
    let config = Config::load();

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, config).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e:#}");
    }

    Ok(())
}

/// Prompt the user for their Canvas API token outside the TUI (raw mode
/// suspended).  Returns the trimmed token string.
fn prompt_api_token(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<String> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    println!();
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║   Canvas API token is missing or expired.           ║");
    println!("║   Generate a new token in Canvas:                   ║");
    println!("║   Account → Settings → New Access Token             ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();
    print!("Paste your API token: ");
    io::stdout().flush()?;

    let mut token = String::new();
    io::stdin().read_line(&mut token)?;
    let token = token.trim().to_string();

    // Restore the TUI.
    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.clear()?;

    Ok(token)
}

/// Prompt for the Canvas base URL outside the TUI.
fn prompt_canvas_url(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<String> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    println!();
    println!("╔══════════════════════════════════════════════════════╗");
    println!("║   No configuration found.                           ║");
    println!("║   Let's set up your Canvas connection.              ║");
    println!("╚══════════════════════════════════════════════════════╝");
    println!();
    print!("Canvas URL (e.g. https://school.instructure.com): ");
    io::stdout().flush()?;

    let mut url = String::new();
    io::stdin().read_line(&mut url)?;
    let url = url.trim().to_string();

    enable_raw_mode()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    terminal.clear()?;

    Ok(url)
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config_result: Result<Config>,
) -> Result<()> {
    // Resolve config — if missing, prompt interactively for URL + token.
    let config = match config_result {
        Ok(cfg) => cfg,
        Err(_) => {
            // No config at all. If we have cache, show it while we prompt.
            if let Some(cached) = cache::load_cache() {
                // We need a placeholder client; we'll replace it after prompting.
                let url = prompt_canvas_url(terminal)?;
                let token = prompt_api_token(terminal)?;
                let cfg = Config {
                    canvas_url: url,
                    api_token: token,
                };
                let _ = cfg.save();

                let client = CanvasClient::new(&cfg.canvas_url, &cfg.api_token)?;
                let mut app = App::new(client);
                app.load_from_cache(cached);
                app.start_fetch();
                app.status_message = "Config saved — syncing with new token…".into();
                return run_main_loop(terminal, app, cfg).await;
            }

            // No cache either — must prompt.
            let url = prompt_canvas_url(terminal)?;
            let token = prompt_api_token(terminal)?;
            let cfg = Config {
                canvas_url: url,
                api_token: token,
            };
            let _ = cfg.save();
            cfg
        }
    };

    let client = CanvasClient::new(&config.canvas_url, &config.api_token)?;
    let mut app = App::new(client);

    // Show cached data instantly, then kick off a background sync.
    if let Some(cached) = cache::load_cache() {
        app.load_from_cache(cached);
        app.start_fetch();
        app.status_message = "Showing cached data — syncing in background…".into();
    } else {
        app.start_fetch();
    }

    run_main_loop(terminal, app, config).await
}

async fn run_main_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    mut app: App,
    mut config: Config,
) -> Result<()> {
    terminal.draw(|f| tui::ui::render(f, &mut app))?;

    loop {
        app.frame_count = app.frame_count.wrapping_add(1);
        terminal.draw(|f| tui::ui::render(f, &mut app))?;

        if let Some(event) = tui::event::poll_event(Duration::from_millis(100))? {
            if let Event::Key(KeyEvent {
                code, modifiers, ..
            }) = event
            {
                tui::event::handle_key(&mut app, code, modifiers);
            }
        }

        if !app.running {
            break;
        }

        // ── $EDITOR launch (text-entry submissions) ───────────────────
        if app.launch_editor {
            app.launch_editor = false;

            let tmp_path = std::env::temp_dir().join("canvas-tui-submission.txt");

            // Suspend the TUI so the editor owns the terminal.
            disable_raw_mode()?;
            execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
            terminal.show_cursor()?;

            let editor = std::env::var("VISUAL")
                .or_else(|_| std::env::var("EDITOR"))
                .unwrap_or_else(|_| "nano".into());

            let _ = std::process::Command::new(&editor)
                .arg(&tmp_path)
                .status();

            let content = std::fs::read_to_string(&tmp_path).unwrap_or_default();
            let _ = std::fs::remove_file(&tmp_path);

            // Restore the TUI.
            enable_raw_mode()?;
            execute!(terminal.backend_mut(), EnterAlternateScreen)?;
            terminal.clear()?;

            if content.trim().is_empty() {
                app.submission_state = SubmissionState::TypePicker;
                app.status_message =
                    "Editor closed with no content — submission cancelled.".into();
            } else {
                app.submission_input = content;
                app.submission_state = SubmissionState::TextPreview;
            }
        }

        // Apply completed fetch/submission/course-detail results without blocking.
        app.poll_fetch_result();
        app.poll_submission_result();
        app.poll_course_pages();
        app.poll_course_detail();

        // ── Re-authentication prompt ──────────────────────────────────
        if app.needs_reauth {
            app.needs_reauth = false;

            let token = prompt_api_token(terminal)?;
            if token.is_empty() {
                app.status_message =
                    "No token entered — still using cached data. Press r to retry.".into();
            } else {
                config.api_token = token;
                let _ = config.save();

                match CanvasClient::new(&config.canvas_url, &config.api_token) {
                    Ok(new_client) => {
                        app.client = new_client;
                        app.status_message = "Token updated — syncing…".into();
                        app.start_fetch();
                    }
                    Err(e) => {
                        app.status_message = format!("Bad config: {e}");
                    }
                }
            }
        }

        if app.needs_refresh {
            app.needs_refresh = false;
            app.start_fetch();
        }
    }

    Ok(())
}
