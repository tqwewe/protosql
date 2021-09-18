use clap::{AppSettings, Clap};

/// Validate protobuf messages with postgres tables.
/// If --dir is specified, each proto file will be read in the directory,
/// with the assumption that there is a proto message with the same name as the file name (as CameCase).
/// The cli will then check for a table with that same message name (as snake_case).
#[derive(Clap, Debug)]
#[clap(
    name = "protosql",
    // version = "1.0",
    author = "Ari Seyhun <ariseyhun@live.com.au>"
)]
#[clap(setting = AppSettings::ColoredHelp)]
pub struct Protosql {
    /// Postgres database URI
    #[clap(short, long)]
    pub uri: String,

    /// Postgres schema. Uses proto's package field if omitted, or 'public' if no package was found in the proto file
    #[clap(short, long)]
    pub schema: Option<String>,

    /// Postgres database table name
    #[clap(short, long)]
    pub table: Option<String>,

    /// Directory of proto files
    #[clap(short, long)]
    pub dir: Option<String>,

    /// Proto file
    #[clap(short, long)]
    pub file: Option<String>,

    /// Message name to check against database table
    #[clap(short, long)]
    pub message: Option<String>,

    /// Print more information
    #[clap(short, long)]
    pub verbose: bool,

    /// Only print errors and warnings
    #[clap(short, long)]
    pub quiet: bool,
}
