use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ghostty-styles", about = "Browse, preview, and cycle Ghostty themes")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage theme collections
    Collection {
        #[command(subcommand)]
        action: CollectionAction,
    },
    /// Apply the next theme from the active collection
    Next,
    /// Manage the cycling daemon
    Cycle {
        #[command(subcommand)]
        action: CycleAction,
    },
    /// Create a new theme
    Create {
        /// Fork from an existing theme by slug
        #[arg(long)]
        from: Option<String>,
    },
    /// Set dark/light mode preference
    Mode {
        #[command(subcommand)]
        action: ModeAction,
    },
}

#[derive(Subcommand)]
pub enum ModeAction {
    /// Set mode to dark (only dark themes)
    Dark,
    /// Set mode to light (only light themes)
    Light,
    /// Auto-detect from OS dark mode setting
    AutoOs,
    /// Auto-switch based on time of day
    AutoTime {
        /// Time to switch to dark themes (HH:MM, default 19:00)
        #[arg(long, default_value = "19:00")]
        dark_after: String,
        /// Time to switch to light themes (HH:MM, default 07:00)
        #[arg(long, default_value = "07:00")]
        light_after: String,
    },
    /// Disable mode filtering
    Off,
    /// Show current mode status
    Status,
}

#[derive(Subcommand)]
pub enum CollectionAction {
    /// Create a new collection
    Create { name: String },
    /// List all collections
    List,
    /// Show themes in a collection
    Show { name: String },
    /// Add a theme by slug to a collection
    Add { collection: String, slug: String },
    /// Set a collection as active
    Use { name: String },
    /// Delete a collection
    Delete { name: String },
}

#[derive(Subcommand)]
pub enum CycleAction {
    /// Start the cycling daemon
    Start,
    /// Stop the cycling daemon
    Stop,
    /// Show daemon status
    Status,
}
