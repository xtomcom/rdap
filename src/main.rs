//! RDAP command-line client

use clap::{Parser, ValueEnum};
use rdap::{display::RdapDisplay, RdapClient, RdapRequest, QueryType};
use std::process;
use colored::Colorize;

#[derive(Parser)]
#[command(name = "rdap")]
#[command(author, version, about = "Modern RDAP client", long_about = None)]
struct Cli {
    /// Query string (domain, IP, AS number, etc.)
    query: Option<String>,

    /// RDAP server URL (optional, uses bootstrap if not specified)
    #[arg(short, long)]
    server: Option<String>,

    /// Query type (auto-detected if not specified)
    #[arg(short = 't', long)]
    query_type: Option<QueryTypeArg>,

    /// Output format
    #[arg(short = 'f', long, default_value = "text")]
    format: OutputFormat,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Timeout in seconds
    #[arg(long, default_value = "30")]
    timeout: u64,

    /// Disable SSL certificate verification
    #[arg(short = 'k', long)]
    insecure: bool,
}

#[derive(Debug, Clone, ValueEnum)]
enum QueryTypeArg {
    Domain,
    Ip,
    Autnum,
    Entity,
    Nameserver,
    Help,
    DomainSearch,
    DomainSearchByNameserver,
    DomainSearchByNameserverIp,
    NameserverSearch,
    NameserverSearchByIp,
    EntitySearch,
    EntitySearchByHandle,
}

impl From<QueryTypeArg> for QueryType {
    fn from(arg: QueryTypeArg) -> Self {
        match arg {
            QueryTypeArg::Domain => QueryType::Domain,
            QueryTypeArg::Ip => QueryType::Ip,
            QueryTypeArg::Autnum => QueryType::Autnum,
            QueryTypeArg::Entity => QueryType::Entity,
            QueryTypeArg::Nameserver => QueryType::Nameserver,
            QueryTypeArg::Help => QueryType::Help,
            QueryTypeArg::DomainSearch => QueryType::DomainSearch,
            QueryTypeArg::DomainSearchByNameserver => QueryType::DomainSearchByNameserver,
            QueryTypeArg::DomainSearchByNameserverIp => QueryType::DomainSearchByNameserverIp,
            QueryTypeArg::NameserverSearch => QueryType::NameserverSearch,
            QueryTypeArg::NameserverSearchByIp => QueryType::NameserverSearchByIp,
            QueryTypeArg::EntitySearch => QueryType::EntitySearch,
            QueryTypeArg::EntitySearchByHandle => QueryType::EntitySearchByHandle,
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    JsonPretty,
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("{} {}", "Error:".bright_red().bold(), e);
        process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    use colored::Colorize;

    let query = cli.query.ok_or("Query is required")?;

    // Detect or use specified query type
    let query_type = if let Some(qt) = cli.query_type {
        qt.into()
    } else {
        RdapRequest::detect_type(&query)?
    };

    if cli.verbose {
        eprintln!("{} Query: {}", "→".bright_blue(), query.bright_white());
        eprintln!("{} Type:  {}", "→".bright_blue(), format!("{}", query_type).bright_yellow());
    }

    // Build request
    let mut request = RdapRequest::new(query_type, query);

    if let Some(server_url) = cli.server {
        let url = url::Url::parse(&server_url)?;
        request = request.with_server(url);
        if cli.verbose {
            eprintln!("{} Server: {}", "→".bright_blue(), server_url.bright_green());
        }
    }

    // Create client
    let client = RdapClient::new()?
        .with_timeout(std::time::Duration::from_secs(cli.timeout));

    // Execute query
    if cli.verbose {
        eprintln!("\n{} Querying RDAP server...\n", "⟳".bright_blue());
    }

    let result = client.query(&request).await?;

    // Display result
    match cli.format {
        OutputFormat::Text => {
            result.display(cli.verbose);
        }
        OutputFormat::Json => {
            let json = serde_json::to_string(&result)?;
            println!("{}", json);
        }
        OutputFormat::JsonPretty => {
            let json = serde_json::to_string_pretty(&result)?;
            println!("{}", json);
        }
    }

    Ok(())
}
