mod api;
mod app;
mod cli;
mod collection;
mod config;
mod creator;
mod cycling;
mod daemon;
mod export;
mod preview;
mod shell_hook;
mod theme;
mod ui;

use std::io;
use std::time::Duration;

use clap::Parser;
use crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers,
};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use app::{App, CollectionsMode, InputMode, Screen};
use cli::{Cli, Commands, CollectionAction};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        None => run_tui(),
        Some(cmd) => dispatch_command(cmd),
    }
}

fn dispatch_command(cmd: Commands) {
    match cmd {
        Commands::Collection { action } => handle_collection(action),
        Commands::Next => {
            match cycling::apply_next() {
                Ok(msg) => println!("{}", msg),
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Commands::Cycle { action } => {
            use cli::CycleAction;
            let result = match action {
                CycleAction::Start => daemon::start(),
                CycleAction::Stop => daemon::stop(),
                CycleAction::Status => daemon::status(),
            };
            if let Err(e) = result {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Create { from } => {
            run_tui_create(from);
        }
    }
}

fn handle_collection(action: CollectionAction) {
    match action {
        CollectionAction::Create { name } => {
            match collection::create_collection(&name) {
                Ok(_) => {
                    println!("Created collection '{}'", name);
                    prompt_daemon_and_hook(&name);
                }
                Err(e) => {
                    eprintln!("Error creating collection: {}", e);
                    std::process::exit(1);
                }
            }
        }
        CollectionAction::List => {
            let names = collection::list_collections();
            if names.is_empty() {
                println!("No collections yet. Create one with:");
                println!("  ghostty-styles collection create <name>");
                return;
            }
            let config = collection::load_config();
            let active = config.active_collection.as_deref();
            for name in &names {
                let marker = if active == Some(name.as_str()) { " (active)" } else { "" };
                match collection::load_collection(name) {
                    Ok(col) => {
                        let count = col.themes.len();
                        let theme_word = if count == 1 { "theme" } else { "themes" };
                        println!("  {}{} - {} {}", name, marker, count, theme_word);
                    }
                    Err(_) => {
                        println!("  {}{} - (error loading)", name, marker);
                    }
                }
            }
        }
        CollectionAction::Show { name } => {
            match collection::load_collection(&name) {
                Ok(col) => {
                    let order_str = match col.order {
                        collection::CycleOrder::Sequential => "sequential",
                        collection::CycleOrder::Shuffle => "shuffle",
                    };
                    let interval_str = col
                        .interval
                        .as_deref()
                        .unwrap_or("not set");
                    println!("Collection: {}", col.name);
                    println!("Themes:     {}", col.themes.len());
                    println!("Order:      {}", order_str);
                    println!("Interval:   {}", interval_str);
                    if col.themes.is_empty() {
                        println!();
                        println!("No themes yet. Add one with:");
                        println!("  ghostty-styles collection add {} <slug>", name);
                    } else {
                        println!();
                        for (i, theme) in col.themes.iter().enumerate() {
                            let marker = if i == col.current_index { " <-" } else { "" };
                            println!("  {}. {}{}", i + 1, theme.title, marker);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
        CollectionAction::Add { collection: coll_name, slug } => {
            // Fetch theme from API
            match api::fetch_config_by_id(&slug) {
                Ok(config) => {
                    let theme = collection::CollectionTheme {
                        slug: config.slug,
                        title: config.title.clone(),
                        is_dark: config.is_dark,
                        raw_config: config.raw_config,
                    };
                    match collection::load_collection(&coll_name) {
                        Ok(mut col) => {
                            col.themes.push(theme);
                            match collection::save_collection(&col) {
                                Ok(()) => {
                                    println!(
                                        "Added '{}' to collection '{}'",
                                        config.title, coll_name
                                    );
                                }
                                Err(e) => {
                                    eprintln!("Error saving collection: {}", e);
                                    std::process::exit(1);
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Error: {}", e);
                            std::process::exit(1);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error fetching theme '{}': {}", slug, e);
                    std::process::exit(1);
                }
            }
        }
        CollectionAction::Use { name } => {
            // Verify collection exists
            if let Err(e) = collection::load_collection(&name) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
            let mut config = collection::load_config();
            config.active_collection = Some(name.clone());
            match collection::save_config(&config) {
                Ok(()) => {
                    println!("Active collection set to '{}'", name);
                }
                Err(e) => {
                    eprintln!("Error saving config: {}", e);
                    std::process::exit(1);
                }
            }
        }
        CollectionAction::Delete { name } => {
            match collection::delete_collection(&name) {
                Ok(()) => {
                    // Clear active_collection if it was the deleted one
                    let mut config = collection::load_config();
                    if config.active_collection.as_deref() == Some(&name) {
                        config.active_collection = None;
                        if let Err(e) = collection::save_config(&config) {
                            eprintln!("Warning: collection deleted but failed to update config: {}", e);
                        }
                    }
                    println!("Deleted collection '{}'", name);
                }
                Err(e) => {
                    eprintln!("Error: {}", e);
                    std::process::exit(1);
                }
            }
        }
    }
}

fn prompt_daemon_and_hook(name: &str) {
    use std::io::{self, BufRead, Write};

    // Ask about interval
    print!("Set a cycling interval? (e.g., 30m, 1h, or press Enter to skip): ");
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().lock().read_line(&mut input).is_ok() {
        let trimmed = input.trim();
        if !trimmed.is_empty() {
            if let Ok(mut coll) = collection::load_collection(name) {
                coll.interval = Some(trimmed.to_string());
                let _ = collection::save_collection(&coll);
                println!("Interval set to '{}'", trimmed);
            }
        }
    }

    // Ask about shell hook
    shell_hook::prompt_install();
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
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .expect("Failed to enter alternate screen");
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut app = App::new();
    app.trigger_fetch();

    let result = run_app(&mut terminal, &mut app);

    // Cleanup
    app.cleanup();
    disable_raw_mode().expect("Failed to disable raw mode");
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)
        .expect("Failed to leave alternate screen");

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run_tui_create(from_slug: Option<String>) {
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
    if term_program.to_lowercase() != "ghostty" {
        eprintln!("ghostty-styles requires the Ghostty terminal.");
        std::process::exit(1);
    }

    let source_theme = if let Some(ref slug) = from_slug {
        match api::fetch_config_by_id(slug) {
            Ok(theme) => Some(theme),
            Err(e) => {
                eprintln!("Error fetching theme '{}': {}", slug, e);
                std::process::exit(1);
            }
        }
    } else {
        None
    };

    enable_raw_mode().expect("Failed to enable raw mode");
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
        .expect("Failed to enter alternate screen");
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

    let mut app = App::new();
    match source_theme {
        Some(theme) => {
            app.creator_state = Some(creator::CreatorState::from_theme(&theme));
            app.screen = Screen::Create;
        }
        None => {
            app.creator_state = Some(creator::CreatorState::new("Untitled".to_string()));
            app.screen = Screen::Create;
        }
    }

    let result = run_app(&mut terminal, &mut app);

    app.cleanup();
    disable_raw_mode().expect("Failed to disable raw mode");
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)
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
            Screen::Collections => ui::render_collections(f, app),
            Screen::Create => ui::render_creator(f, app),
            Screen::CreateMeta => ui::render_create_meta(f, app),
        })?;

        // Poll for events with a timeout so we can check background messages
        if event::poll(Duration::from_millis(50))? {
            let ev = event::read()?;
            match ev {
                Event::Key(key) => {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    // Clear status message on any keypress
                    app.status_message = None;

                    // Ctrl+C always quits
                    if key.modifiers.contains(KeyModifiers::CONTROL)
                        && key.code == KeyCode::Char('c')
                    {
                        app.should_quit = true;
                    }

                    match app.screen {
                        Screen::Browse => handle_browse_input(app, key.code),
                        Screen::Detail => handle_detail_input(app, key.code),
                        Screen::Confirm => handle_confirm_input(app, key.code),
                        Screen::Collections => handle_collections_input(app, key.code),
                        Screen::Create => handle_create_input(app, key.code, key.modifiers),
                        Screen::CreateMeta => handle_create_meta_input(app, key.code),
                    }
                }
                Event::Mouse(mouse) => {
                    if app.screen == Screen::Create {
                        handle_create_mouse(app, mouse);
                    }
                }
                _ => {}
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
            KeyCode::Char('n') => app.enter_creator("Untitled".to_string()),
            KeyCode::Char(']') => app.next_page(),
            KeyCode::Char('[') => app.prev_page(),
            KeyCode::Char('p') => app.toggle_osc_preview(),
            KeyCode::Char('a') => {
                if !app.themes.is_empty() {
                    app.screen = Screen::Confirm;
                }
            }
            KeyCode::Char('c') => {
                if !app.themes.is_empty() {
                    app.open_collection_popup();
                }
            }
            KeyCode::Char('C') => {
                app.enter_collections();
            }
            KeyCode::Char('r') => {
                app.trigger_fetch();
            }
            _ => {}
        },
        InputMode::CollectionSelect => match key {
            KeyCode::Char('j') | KeyCode::Down => {
                if !app.collection_names.is_empty() {
                    app.collection_popup_cursor = (app.collection_popup_cursor + 1).min(app.collection_names.len() - 1);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                app.collection_popup_cursor = app.collection_popup_cursor.saturating_sub(1);
            }
            KeyCode::Enter => {
                if let Some(name) = app.collection_names.get(app.collection_popup_cursor).cloned() {
                    app.add_to_collection(&name);
                }
            }
            KeyCode::Char('n') => {
                app.input_mode = InputMode::CollectionCreate;
                app.collection_name_input.clear();
            }
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },
        InputMode::CollectionCreate => match key {
            KeyCode::Enter => {
                app.create_collection_and_add();
            }
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                app.collection_name_input.pop();
            }
            KeyCode::Char(c) => {
                app.collection_name_input.push(c);
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
        KeyCode::Char('c') => {
            app.open_collection_popup();
        }
        KeyCode::Char('f') => {
            app.enter_creator_from_theme();
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

fn handle_collections_input(app: &mut App, key: KeyCode) {
    match app.collections_mode {
        CollectionsMode::Normal => {
            if app.collections_viewing_themes {
                handle_collections_theme_input(app, key);
            } else {
                handle_collections_list_input(app, key);
            }
        }
        CollectionsMode::NewCollection => match key {
            KeyCode::Enter => {
                let name = app.collections_input.trim().to_string();
                if !name.is_empty() {
                    match collection::create_collection(&name) {
                        Ok(_) => {
                            app.status_message = Some(format!("Created collection '{}'", name));
                            app.refresh_collections();
                        }
                        Err(e) => {
                            app.status_message = Some(format!("Error: {}", e));
                        }
                    }
                }
                app.collections_mode = CollectionsMode::Normal;
                app.collections_input.clear();
            }
            KeyCode::Esc => {
                app.collections_mode = CollectionsMode::Normal;
                app.collections_input.clear();
            }
            KeyCode::Backspace => {
                app.collections_input.pop();
            }
            KeyCode::Char(c) => {
                app.collections_input.push(c);
            }
            _ => {}
        },
        CollectionsMode::SetInterval => match key {
            KeyCode::Enter => {
                if let Some(name) = app.collections_list.get(app.collections_cursor).cloned() {
                    if let Ok(mut coll) = collection::load_collection(&name) {
                        let trimmed = app.collections_input.trim().to_string();
                        if trimmed.is_empty() {
                            coll.interval = None;
                            app.status_message = Some(format!("Cleared interval for '{}'", name));
                        } else {
                            coll.interval = Some(trimmed.clone());
                            app.status_message =
                                Some(format!("Set interval '{}' for '{}'", trimmed, name));
                        }
                        if let Err(e) = collection::save_collection(&coll) {
                            app.status_message = Some(format!("Error: {}", e));
                        }
                    }
                }
                app.collections_mode = CollectionsMode::Normal;
                app.collections_input.clear();
            }
            KeyCode::Esc => {
                app.collections_mode = CollectionsMode::Normal;
                app.collections_input.clear();
            }
            KeyCode::Backspace => {
                app.collections_input.pop();
            }
            KeyCode::Char(c) => {
                app.collections_input.push(c);
            }
            _ => {}
        },
        CollectionsMode::ConfirmDelete => match key {
            KeyCode::Char('y') => {
                if let Some(name) = app.collections_list.get(app.collections_cursor).cloned() {
                    match collection::delete_collection(&name) {
                        Ok(()) => {
                            // Clear active if it was the deleted one
                            let mut config = collection::load_config();
                            if config.active_collection.as_deref() == Some(&name) {
                                config.active_collection = None;
                                let _ = collection::save_config(&config);
                            }
                            app.status_message = Some(format!("Deleted collection '{}'", name));
                            app.refresh_collections();
                        }
                        Err(e) => {
                            app.status_message = Some(format!("Error: {}", e));
                        }
                    }
                }
                app.collections_mode = CollectionsMode::Normal;
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                app.collections_mode = CollectionsMode::Normal;
            }
            _ => {}
        },
    }
}

fn handle_collections_list_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            if !app.collections_list.is_empty() {
                app.collections_cursor =
                    (app.collections_cursor + 1).min(app.collections_list.len() - 1);
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.collections_cursor = app.collections_cursor.saturating_sub(1);
        }
        KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => {
            app.load_selected_collection();
        }
        KeyCode::Char('n') => {
            app.collections_mode = CollectionsMode::NewCollection;
            app.collections_input.clear();
        }
        KeyCode::Char('d') => {
            if !app.collections_list.is_empty() {
                app.collections_mode = CollectionsMode::ConfirmDelete;
            }
        }
        KeyCode::Char('u') => {
            if let Some(name) = app.collections_list.get(app.collections_cursor).cloned() {
                let mut config = collection::load_config();
                config.active_collection = Some(name.clone());
                match collection::save_config(&config) {
                    Ok(()) => {
                        app.status_message = Some(format!("Activated collection '{}'", name));
                    }
                    Err(e) => {
                        app.status_message = Some(format!("Error: {}", e));
                    }
                }
            }
        }
        KeyCode::Char('s') => {
            if let Some(name) = app.collections_list.get(app.collections_cursor).cloned() {
                if let Ok(mut coll) = collection::load_collection(&name) {
                    coll.order = match coll.order {
                        collection::CycleOrder::Sequential => collection::CycleOrder::Shuffle,
                        collection::CycleOrder::Shuffle => collection::CycleOrder::Sequential,
                    };
                    let order_label = match coll.order {
                        collection::CycleOrder::Sequential => "sequential",
                        collection::CycleOrder::Shuffle => "shuffle",
                    };
                    match collection::save_collection(&coll) {
                        Ok(()) => {
                            app.status_message =
                                Some(format!("Set '{}' order to {}", name, order_label));
                        }
                        Err(e) => {
                            app.status_message = Some(format!("Error: {}", e));
                        }
                    }
                }
            }
        }
        KeyCode::Char('i') => {
            if !app.collections_list.is_empty() {
                app.collections_mode = CollectionsMode::SetInterval;
                app.collections_input.clear();
            }
        }
        KeyCode::Char('h') | KeyCode::Left | KeyCode::Esc => {
            app.screen = Screen::Browse;
        }
        _ => {}
    }
}

fn handle_collections_theme_input(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Char('j') | KeyCode::Down => {
            if let Some(ref coll) = app.collections_detail {
                if !coll.themes.is_empty() {
                    app.collections_theme_cursor =
                        (app.collections_theme_cursor + 1).min(coll.themes.len() - 1);
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            app.collections_theme_cursor = app.collections_theme_cursor.saturating_sub(1);
        }
        KeyCode::Char('x') => {
            if let Some(name) = app.collections_list.get(app.collections_cursor).cloned() {
                if let Ok(mut coll) = collection::load_collection(&name) {
                    if app.collections_theme_cursor < coll.themes.len() {
                        let removed = coll.themes.remove(app.collections_theme_cursor);
                        // Adjust current_index if needed
                        if coll.themes.is_empty() {
                            coll.current_index = 0;
                        } else if coll.current_index >= coll.themes.len() {
                            coll.current_index = coll.themes.len() - 1;
                        }
                        match collection::save_collection(&coll) {
                            Ok(()) => {
                                app.status_message =
                                    Some(format!("Removed '{}' from '{}'", removed.title, name));
                                // Adjust theme cursor before refreshing detail view
                                let theme_count = coll.themes.len();
                                if theme_count == 0 {
                                    app.collections_theme_cursor = 0;
                                } else if app.collections_theme_cursor >= theme_count {
                                    app.collections_theme_cursor = theme_count - 1;
                                }
                                // Refresh the detail view
                                app.collections_detail = Some(coll);
                            }
                            Err(e) => {
                                app.status_message = Some(format!("Error: {}", e));
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Char('h') | KeyCode::Left | KeyCode::Esc => {
            app.collections_viewing_themes = false;
            app.collections_detail = None;
        }
        _ => {}
    }
}

fn handle_create_input(app: &mut App, key: KeyCode, modifiers: KeyModifiers) {
    use crate::creator::{ColorField, PickerMode, SliderFocus};

    let state = match app.creator_state.as_mut() {
        Some(s) => s,
        None => return,
    };

    if state.editing {
        match state.picker_mode {
            PickerMode::Slider => match key {
                KeyCode::Left => {
                    let delta = if modifiers.contains(KeyModifiers::SHIFT) {
                        -10.0
                    } else {
                        -1.0
                    };
                    state.adjust_slider(delta);
                    if state.osc_preview {
                        let config = state.build_preview_config();
                        preview::apply_osc_preview(&config);
                    }
                }
                KeyCode::Right => {
                    let delta = if modifiers.contains(KeyModifiers::SHIFT) {
                        10.0
                    } else {
                        1.0
                    };
                    state.adjust_slider(delta);
                    if state.osc_preview {
                        let config = state.build_preview_config();
                        preview::apply_osc_preview(&config);
                    }
                }
                KeyCode::Up => {
                    state.slider_focus = match state.slider_focus {
                        SliderFocus::Hue => SliderFocus::Lightness,
                        SliderFocus::Saturation => SliderFocus::Hue,
                        SliderFocus::Lightness => SliderFocus::Saturation,
                    };
                }
                KeyCode::Down => {
                    state.slider_focus = match state.slider_focus {
                        SliderFocus::Hue => SliderFocus::Saturation,
                        SliderFocus::Saturation => SliderFocus::Lightness,
                        SliderFocus::Lightness => SliderFocus::Hue,
                    };
                }
                KeyCode::Tab => {
                    state.picker_mode = PickerMode::HexInput;
                    state.sync_hex_from_color();
                }
                KeyCode::Esc | KeyCode::Enter => {
                    state.editing = false;
                }
                _ => {}
            },
            PickerMode::HexInput => match key {
                KeyCode::Char(c) if c.is_ascii_hexdigit() || c == '#' => {
                    if state.hex_input.len() < 7 {
                        state.hex_input.push(c);
                    }
                    // Auto-commit when we have a '#' plus 6 hex digits
                    let digits: String = state
                        .hex_input
                        .chars()
                        .filter(|ch| ch.is_ascii_hexdigit())
                        .collect();
                    if digits.len() == 6 {
                        state.commit_hex_input();
                        if state.osc_preview {
                            let config = state.build_preview_config();
                            preview::apply_osc_preview(&config);
                        }
                    }
                }
                KeyCode::Backspace => {
                    state.hex_input.pop();
                }
                KeyCode::Enter => {
                    state.commit_hex_input();
                    state.editing = false;
                    if state.osc_preview {
                        let config = state.build_preview_config();
                        preview::apply_osc_preview(&config);
                    }
                }
                KeyCode::Tab => {
                    state.picker_mode = PickerMode::Slider;
                }
                KeyCode::Esc => {
                    state.editing = false;
                }
                _ => {}
            },
        }
    } else {
        // Navigation mode
        let field_count = ColorField::all().len();
        let visible_rows: usize = 18;

        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                if state.field_index < field_count - 1 {
                    state.field_index += 1;
                    if state.field_index >= state.field_scroll + visible_rows {
                        state.field_scroll = state.field_index + 1 - visible_rows;
                    }
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if state.field_index > 0 {
                    state.field_index -= 1;
                    if state.field_index < state.field_scroll {
                        state.field_scroll = state.field_index;
                    }
                }
            }
            KeyCode::Enter | KeyCode::Char('l') => {
                state.editing = true;
                state.slider_focus = SliderFocus::Hue;
                state.sync_hex_from_color();
            }
            KeyCode::Char('g') => {
                state.gen_algorithm = state.gen_algorithm.toggle();
                state.generate_palette();
                app.status_message =
                    Some(format!("Algorithm: {}", state.gen_algorithm.label()));
                if state.osc_preview {
                    let config = state.build_preview_config();
                    preview::apply_osc_preview(&config);
                }
            }
            KeyCode::Char('p') => {
                if state.osc_preview {
                    // Turn off: restore colors
                    if let Some(ref saved) = app.saved_colors {
                        preview::restore_colors(saved);
                    }
                    app.saved_colors = None;
                    state.osc_preview = false;
                    app.status_message = Some("Preview off - colors restored".into());
                } else {
                    // Turn on: save current and apply
                    app.saved_colors = Some(preview::save_current_colors());
                    state.osc_preview = true;
                    let config = state.build_preview_config();
                    preview::apply_osc_preview(&config);
                    app.status_message = Some("Live preview on".into());
                }
            }
            KeyCode::Char('s') => {
                app.enter_create_meta();
            }
            KeyCode::Esc | KeyCode::Char('q') => {
                // Restore OSC if active
                if state.osc_preview {
                    if let Some(ref saved) = app.saved_colors {
                        preview::restore_colors(saved);
                    }
                    app.saved_colors = None;
                }
                app.creator_state = None;
                app.screen = Screen::Browse;
            }
            _ => {}
        }
    }
}

fn handle_create_meta_input(app: &mut App, key: KeyCode) {
    // First check if we're in editing mode
    let is_editing = app
        .create_meta_state
        .as_ref()
        .map_or(false, |m| m.editing);
    let field_index = app.create_meta_state.as_ref().map_or(0, |m| m.field_index);

    if is_editing {
        let meta = match app.create_meta_state.as_mut() {
            Some(m) => m,
            None => return,
        };
        match field_index {
            0 => {
                // Editing title -- need to edit creator_state.title
                // For Enter/Esc we can toggle editing off via meta.
                // For char/backspace we must drop meta first and edit creator_state.
                match key {
                    KeyCode::Enter | KeyCode::Esc => {
                        meta.editing = false;
                    }
                    _ => {
                        let _ = meta;
                        match key {
                            KeyCode::Char(c) => {
                                if let Some(ref mut creator) = app.creator_state {
                                    creator.title.push(c);
                                }
                            }
                            KeyCode::Backspace => {
                                if let Some(ref mut creator) = app.creator_state {
                                    creator.title.pop();
                                }
                            }
                            _ => {}
                        }
                        return;
                    }
                }
            }
            1 => {
                // Editing description
                match key {
                    KeyCode::Char(c) => meta.description.push(c),
                    KeyCode::Backspace => {
                        meta.description.pop();
                    }
                    KeyCode::Enter | KeyCode::Esc => {
                        meta.editing = false;
                    }
                    _ => {}
                }
            }
            2 => {
                // Tag selection mode
                let tag_list = [
                    "dark",
                    "light",
                    "minimal",
                    "colorful",
                    "retro",
                    "pastel",
                    "high-contrast",
                    "monochrome",
                    "warm",
                    "cool",
                    "neon",
                ];
                match key {
                    KeyCode::Char('j') | KeyCode::Down => {
                        meta.tag_cursor = (meta.tag_cursor + 1).min(tag_list.len() - 1);
                    }
                    KeyCode::Char('k') | KeyCode::Up => {
                        meta.tag_cursor = meta.tag_cursor.saturating_sub(1);
                    }
                    KeyCode::Char(' ') | KeyCode::Enter => {
                        let tag = tag_list[meta.tag_cursor].to_string();
                        if meta.tags.contains(&tag) {
                            meta.tags.retain(|t| t != &tag);
                        } else if meta.tags.len() < 5 {
                            meta.tags.push(tag);
                        }
                    }
                    KeyCode::Esc => {
                        meta.editing = false;
                    }
                    _ => {}
                }
            }
            3 => {
                // Editing author name
                match key {
                    KeyCode::Char(c) => meta.author_name.push(c),
                    KeyCode::Backspace => {
                        meta.author_name.pop();
                    }
                    KeyCode::Enter | KeyCode::Esc => {
                        meta.editing = false;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    } else {
        // Navigation mode -- no persistent mutable borrow on meta needed for actions
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(ref mut meta) = app.create_meta_state {
                    meta.field_index = (meta.field_index + 1).min(4);
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(ref mut meta) = app.create_meta_state {
                    meta.field_index = meta.field_index.saturating_sub(1);
                }
            }
            KeyCode::Enter => {
                if let Some(ref mut meta) = app.create_meta_state {
                    if meta.field_index <= 3 {
                        meta.editing = true;
                    }
                }
            }
            KeyCode::Char('a') => {
                // Apply to Ghostty config
                if let Some(ref state) = app.creator_state {
                    if state.title.trim().is_empty() {
                        app.status_message = Some("Title cannot be empty".into());
                    } else {
                        match export::apply_created_theme(state) {
                            Ok(path) => {
                                app.status_message = Some(format!("Applied to {}", path));
                            }
                            Err(e) => {
                                app.status_message = Some(format!("Error: {}", e));
                            }
                        }
                    }
                }
            }
            KeyCode::Char('e') => {
                // Export to file
                if let Some(ref state) = app.creator_state {
                    if state.title.trim().is_empty() {
                        app.status_message = Some("Title cannot be empty".into());
                    } else {
                        match export::export_theme(state) {
                            Ok(path) => {
                                app.status_message = Some(format!("Exported to {}", path));
                            }
                            Err(e) => {
                                app.status_message = Some(format!("Error: {}", e));
                            }
                        }
                    }
                }
            }
            KeyCode::Char('u') => {
                // Upload
                if let Some(ref state) = app.creator_state {
                    if state.title.trim().is_empty() {
                        app.status_message = Some("Title cannot be empty".into());
                    } else {
                        match export::upload_theme(state) {
                            Ok(msg) => {
                                app.status_message = Some(msg);
                            }
                            Err(e) => {
                                app.status_message = Some(format!("Error: {}", e));
                            }
                        }
                    }
                }
            }
            KeyCode::Esc => {
                // Back to creator
                app.create_meta_state = None;
                app.screen = Screen::Create;
            }
            _ => {}
        }
    }
}

fn app_area(_app: &App) -> ratatui::layout::Rect {
    let (w, h) = crossterm::terminal::size().unwrap_or((80, 24));
    ratatui::layout::Rect::new(0, 0, w, h)
}

fn handle_create_mouse(app: &mut App, mouse: crossterm::event::MouseEvent) {
    use crossterm::event::MouseEventKind;

    match mouse.kind {
        MouseEventKind::Down(_) | MouseEventKind::Drag(_) => {}
        _ => return,
    }

    let area = app_area(app);
    let layout = ui::creator::get_layout_rects(area);
    let col = mouse.column;
    let row = mouse.row;

    let fields_inner = layout.fields_inner;
    let picker_inner = layout.picker_inner;

    // Click on the field list area
    if col >= fields_inner.x
        && col < fields_inner.x + fields_inner.width
        && row >= fields_inner.y
        && row < fields_inner.y + fields_inner.height
    {
        let state = match app.creator_state.as_mut() {
            Some(s) => s,
            None => return,
        };

        // Reserve one line at the bottom for the algorithm indicator (same as render_field_list)
        let list_height = (fields_inner.height as usize).saturating_sub(1);
        let row_in_panel = (row - fields_inner.y) as usize;

        if row_in_panel < list_height {
            let field_idx = row_in_panel + state.field_scroll;
            let total_fields = creator::ColorField::all().len();
            if field_idx < total_fields {
                state.field_index = field_idx;
                state.editing = true;
                state.sync_hex_from_color();
            }
        }
        return;
    }

    // Click/drag on the picker area (only when editing in slider mode)
    if col >= picker_inner.x
        && col < picker_inner.x + picker_inner.width
        && row >= picker_inner.y
        && row < picker_inner.y + picker_inner.height
    {
        let state = match app.creator_state.as_mut() {
            Some(s) => s,
            None => return,
        };

        if !state.editing {
            return;
        }

        // The slider rows are laid out with spacers between them:
        //   row 0: Hue
        //   row 1: spacer
        //   row 2: Saturation
        //   row 3: spacer
        //   row 4: Lightness
        let rel_row = (row - picker_inner.y) as usize;
        let slider_row = match rel_row {
            0 => Some(0), // Hue
            2 => Some(1), // Saturation
            4 => Some(2), // Lightness
            _ => None,
        };

        if let Some(slider_idx) = slider_row {
            // Match the slider bar geometry from render_slider_row:
            //   prefix_len = 5, suffix_len = 6
            //   bar starts at picker_inner.x + 5
            //   bar_width = picker_inner.width - 5 - 6
            let prefix_len: u16 = 5;
            let suffix_len: u16 = 6;
            let bar_start = picker_inner.x + prefix_len;
            let bar_width = picker_inner.width.saturating_sub(prefix_len + suffix_len);

            if bar_width == 0 {
                return;
            }

            if col >= bar_start && col < bar_start + bar_width {
                let frac = (col - bar_start) as f64 / (bar_width as f64 - 1.0).max(1.0);
                let frac = frac.clamp(0.0, 1.0);

                let mut color = state.colors[state.field_index];
                match slider_idx {
                    0 => color.h = (frac * 360.0).rem_euclid(360.0),
                    1 => color.s = (frac * 100.0).clamp(0.0, 100.0),
                    2 => color.l = (frac * 100.0).clamp(0.0, 100.0),
                    _ => unreachable!(),
                }

                state.colors[state.field_index] = color;
                state.unsaved = true;
                state.palette_dirty = true;
                state.sync_hex_from_color();

                // Update slider focus to match what was clicked
                state.slider_focus = match slider_idx {
                    0 => creator::SliderFocus::Hue,
                    1 => creator::SliderFocus::Saturation,
                    _ => creator::SliderFocus::Lightness,
                };

                if state.osc_preview {
                    let config = state.build_preview_config();
                    preview::apply_osc_preview(&config);
                }
            }
        }
    }
}
