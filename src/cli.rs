use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    version,
    about = "Inspect Suzerain dumps through indexed queries or HTML."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    /// Import legacy dump files/directories atomically into a local SQLite index.
    Import {
        #[arg(required = true, num_args = 1..)]
        inputs: Vec<PathBuf>,
        #[command(flatten)]
        output: Output,
        /// Replace the contents of an existing Data Viewer index.
        #[arg(long)]
        replace: bool,
    },
    /// Report imported sources and their independent provenance.
    Sources {
        #[command(flatten)]
        read: Read,
        #[command(flatten)]
        page: Page,
    },
    /// Check indexed identities, links and optionally original file hashes.
    Validate {
        #[command(flatten)]
        read: Read,
        #[command(flatten)]
        page: Page,
        #[arg(long)]
        check_files: bool,
    },
    /// Search titles, dialogue, menu text and scripts. Omit text to list records.
    Search {
        text: Option<String>,
        #[arg(long, value_enum, default_value = "all")]
        kind: SearchKind,
        #[command(flatten)]
        read: Read,
        #[command(flatten)]
        page: Page,
    },
    /// Get a dialogue node, including exact scripts and outgoing links.
    Node {
        #[command(flatten)]
        selector: NodeSelector,
        #[command(flatten)]
        read: Read,
        /// Return the original decoded JSON record instead of a compact view.
        #[arg(long)]
        raw: bool,
    },
    /// Traverse the source dump's graph, retaining unresolved links.
    Neighbors {
        #[command(flatten)]
        selector: NodeSelector,
        #[command(flatten)]
        read: Read,
        #[command(flatten)]
        page: Page,
        #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u8).range(1..=5))]
        depth: u8,
        #[arg(long, value_enum, default_value = "both")]
        direction: Direction,
    },
    /// Find literal references in Lua, articy instructions and sequences.
    References {
        variable: String,
        #[arg(long, value_enum)]
        language: Option<Language>,
        #[command(flatten)]
        read: Read,
        #[command(flatten)]
        page: Page,
    },
    /// Get a named entity and its game-flow/dialogue relationships.
    Entity {
        name: String,
        /// Optional kind, e.g. DecisionData, BillData, ConversationData or GameFlowData.
        #[arg(long)]
        kind: Option<String>,
        #[arg(long)]
        raw: bool,
        #[command(flatten)]
        read: Read,
    },
    /// Generate HTML without the interactive picker.
    Html {
        #[arg(long)]
        input: PathBuf,
        #[arg(long, default_value = "out")]
        output: PathBuf,
        /// Replace generated files only in a previously marked Viewer output directory.
        #[arg(long)]
        overwrite: bool,
    },
}

#[derive(Args)]
pub struct Output {
    #[arg(long, default_value = "suzerain-data.sqlite")]
    pub database: PathBuf,
    #[arg(long, value_enum, default_value = "json")]
    pub format: Format,
    /// Write the response to a new file instead of stdout; existing files are refused.
    #[arg(long)]
    pub output: Option<PathBuf>,
    #[arg(long, default_value_t = 262144, value_parser = clap::value_parser!(u64).range(1024..=16777216))]
    pub max_bytes: u64,
}

#[derive(Args)]
pub struct Read {
    #[command(flatten)]
    pub output: Output,
    /// Source ID from the sources command; use it to disambiguate captures.
    #[arg(long, value_parser = clap::value_parser!(i64).range(1..))]
    pub source: Option<i64>,
    #[arg(long)]
    pub pack: Option<String>,
}

#[derive(Args)]
pub struct Page {
    #[arg(long, default_value_t = 20, value_parser = clap::value_parser!(u32).range(1..=200))]
    pub limit: u32,
    /// Opaque next_cursor returned by the same query on the same index revision.
    #[arg(long)]
    pub cursor: Option<String>,
}

#[derive(Args)]
pub struct NodeSelector {
    #[arg(long)]
    pub conversation_id: Option<i64>,
    #[arg(
        long,
        requires = "conversation_id",
        required_unless_present = "articy_id"
    )]
    pub entry_id: Option<i64>,
    #[arg(long, conflicts_with = "entry_id")]
    pub articy_id: Option<String>,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Format {
    Json,
    Text,
    Markdown,
}
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum SearchKind {
    All,
    Conversations,
    Nodes,
    Entities,
}
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Direction {
    Incoming,
    Outgoing,
    Both,
}
#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum Language {
    Lua,
    Articy,
    Sequence,
    Variable,
}

impl Language {
    pub fn name(self) -> &'static str {
        match self {
            Self::Lua => "lua",
            Self::Articy => "articy",
            Self::Sequence => "sequence",
            Self::Variable => "variable",
        }
    }
}
