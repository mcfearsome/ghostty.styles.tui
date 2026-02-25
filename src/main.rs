mod api;
mod app;
mod cli;
mod config;
mod preview;
mod theme;
mod ui;

use std::io;
use std::time::Duration;

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{App, InputMode, Screen};
use cli::{Cli, Commands, CollectionAction, CycleAction};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        None => run_tui(),
        Some(cmd) => dispatch_command(cmd),
    }
}

fn dispatch_command(cmd: Commands) {
    match cmd {
        Commands::Collection { action } => match action {
            CollectionAction::Create { name } => {
                eprintln!("collection create '{}': not yet implemented", name);
            }
            CollectionAction::List => {
                eprintln!("collection list: not yet implemented");
            }
            CollectionAction::Show { name } => {
                eprintln!("collection show '{}': not yet implemented", name);
            }
            CollectionAction::Add { collection, slug } => {
                eprintln!(
                    "collection add '{}' to '{}': not yet implemented",
                    slug, collection
                );
            }
            CollectionAction::Use { name } => {
                eprintln!("collection use '{}': not yet implemented", name);
            }
            CollectionAction::Delete { name } => {
                eprintln!("collection delete '{}': not yet implemented", name);
            }
        },
        Commands::Next => {
            eprintln!("next: not yet implemented");
        }
        Commands::Cycle { action } => match action {
            CycleAction::Start => {
                eprintln!("cycle start: not yet implemented");
            }
            CycleAction::Stop => {
                eprintln!("cycle stop: not yet implemented");
            }
            CycleAction::Status => {
                eprintln!("cycle status: not yet implemented");
            }
        },
    }
}

fn run_tui() {
    // Ghostty detection
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
    if term_program.to_lowercase() != "ghostty" {
        eprintln!();
        eprintln!(
            "  \x1b[1;35mghostty.styles\x1b[0m requires the \x1b[1mGhostty\x1b[0m terminal."
        );
        eprintln!();
        eprintln!(
            "  Detected terminal: \x1b[33m{}\x1b[0m",
            if term_program.is_empty() {
                "unknown"
            } else {
                &term_program
            }
        );
        eprintln!("  Get Ghostty at: \x1b[4;36mhttps://ghostty.org\x1b[0m");
        eprintln!();
        std::process::exit(1);
    }

    // Setup terminal
    enable_raw_mode().expect("Failed to enable raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).expect("Failed to enter alternate screen");
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut app = App::new();
    app.trigger_fetch();

    let result = run_app(&mut terminal, &mut app);

    // Cleanup
    app.cleanup();
    disable_raw_mode().expect("Failed to disable raw mode");
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .expect("Failed to leave alternate screen");

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<(), io::Error> {
    loop {
        app.poll_background();

        terminal.draw(|f| match app.screen {
            Screen::Browse => ui::render_browser(f, app),
            Screen::Detail | Screen::Confirm => ui::render_detail(f, app),
        })?;

        // Poll for events with a timeout so we can check background messages
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Clear status message on any keypress
                app.status_message = None;

                // Ctrl+C always quits
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c')
                {
                    app.should_quit = true;
                }

                match app.screen {
                    Screen::Browse => handle_browse_input(app, key.code),
                    Screen::Detail => handle_detail_input(app, key.code),
                    Screen::Confirm => handle_confirm_input(app, key.code),
                }
            }
        }

        if app.should_quit {
            return Ok(());
        }
    }
}

fn handle_browse_input(app: &mut App, key: KeyCode) {
    match app.input_mode {
        InputMode::Search => match key {
            KeyCode::Enter => app.submit_search(),
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
                app.search_input.clear();
            }
            KeyCode::Backspace => {
                app.search_input.pop();
            }
            KeyCode::Char(c) => {
                app.search_input.push(c);
            }
            _ => {}
        },
        InputMode::TagSelect => match key {
            KeyCode::Char('j') | KeyCode::Down => {
                app.tag_cursor = (app.tag_cursor + 1).min(app::AVAILABLE_TAGS.len() - 1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.tag_cursor = app.tag_cursor.saturating_sub(1);
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                app.select_tag();
            }
            KeyCode::Esc | KeyCode::Char('t') => {
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },
        InputMode::Normal => match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                app.should_quit = true;
            }
            KeyCode::Char('j') | KeyCode::Down => app.select_next(),
            KeyCode::Char('k') | KeyCode::Up => app.select_prev(),
            KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => {
                if !app.themes.is_empty() {
                    app.screen = Screen::Detail;
                }
            }
            KeyCode::Char('/') => {
                app.input_mode = InputMode::Search;
                app.search_input = app.active_query.clone().unwrap_or_default();
            }
            KeyCode::Char('t') => {
                app.input_mode = InputMode::TagSelect;
            }
            KeyCode::Char('s') => app.cycle_sort(),
            KeyCode::Char('d') => app.toggle_dark_filter(),
            KeyCode::Char('n') => app.next_page(),
            KeyCode::Char('N') => app.prev_page(),
            KeyCode::Char('p') => app.toggle_osc_preview(),
            KeyCode::Char('a') => {
                if !app.themes.is_empty() {
                    app.screen = Screen::Confirm;
                }
            }
            KeyCode::Char('r') => {
                app.trigger_fetch();
            }
            _ => {}
        },
    }
}

fn handle_detail_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('h') | KeyCode::Left => {
            app.screen = Screen::Browse;
        }
        KeyCode::Char('p') => app.toggle_osc_preview(),
        KeyCode::Char('a') => {
            app.screen = Screen::Confirm;
        }
        _ => {}
    }
}

fn handle_confirm_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('y') | KeyCode::Enter => {
            app.apply_theme();
        }
        KeyCode::Char('n') | KeyCode::Esc => {
            app.screen = Screen::Detail;
        }
        _ => {}
    }
}
