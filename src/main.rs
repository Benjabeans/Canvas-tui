mod api;
mod cache;
mod config;
mod models;
mod tui;

use anyhow::{Context, Result};
use crossterm::{
    event::{Event, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
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

    let config = Config::load().with_context(|| {
        "Failed to load configuration.\n\
         Run `canvas-tui --init` to generate a config file,\n\
         or set CANVAS_URL and CANVAS_API_TOKEN environment variables."
    })?;

    let client = CanvasClient::new(&config.canvas_url, &config.api_token)?;

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal, client).await;

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e:#}");
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    client: CanvasClient,
) -> Result<()> {
    let mut app = App::new(client);

    // Show cached data instantly, then kick off a background sync.
    if let Some(cached) = cache::load_cache() {
        app.load_from_cache(cached);
        app.start_fetch();
        app.status_message = "Showing cached data — syncing in background…".into();
    } else {
        app.start_fetch();
    }
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

        // Apply completed fetch/submission results without blocking.
        app.poll_fetch_result();
        app.poll_submission_result();

        if app.needs_refresh {
            app.needs_refresh = false;
            app.start_fetch();
        }
    }

    Ok(())
}
