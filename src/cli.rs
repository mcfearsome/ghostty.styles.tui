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
