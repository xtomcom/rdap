//! RDAP command-line client

use clap::{Parser, ValueEnum};
use colored::Colorize;
use rdap::{QueryType, RdapClient, RdapRequest, display::RdapDisplay};
use std::process;

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

    /// Disable following registrar referrals for domain queries
    #[arg(long)]
    no_referral: bool,

    /// JSON output source: registry or registrar (default: registrar)
    #[arg(long, default_value = "registrar")]
    json_source: JsonSource,

    /// Update configuration files from GitHub (config.json and tlds.json)
    #[arg(short = 'u', long)]
    update: bool,
}

#[derive(Debug, Clone, ValueEnum)]
enum QueryTypeArg {
    Domain,
    Tld,
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
            QueryTypeArg::Domain => Self::Domain,
            QueryTypeArg::Tld => Self::Tld,
            QueryTypeArg::Ip => Self::Ip,
            QueryTypeArg::Autnum => Self::Autnum,
            QueryTypeArg::Entity => Self::Entity,
            QueryTypeArg::Nameserver => Self::Nameserver,
            QueryTypeArg::Help => Self::Help,
            QueryTypeArg::DomainSearch => Self::DomainSearch,
            QueryTypeArg::DomainSearchByNameserver => Self::DomainSearchByNameserver,
            QueryTypeArg::DomainSearchByNameserverIp => Self::DomainSearchByNameserverIp,
            QueryTypeArg::NameserverSearch => Self::NameserverSearch,
            QueryTypeArg::NameserverSearchByIp => Self::NameserverSearchByIp,
            QueryTypeArg::EntitySearch => Self::EntitySearch,
            QueryTypeArg::EntitySearchByHandle => Self::EntitySearchByHandle,
        }
    }
}

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
    JsonPretty,
}

#[derive(Debug, Clone, ValueEnum)]
enum JsonSource {
    /// Use registry data (from TLD registry like Verisign)
    Registry,
    /// Use registrar data if available, fallback to registry (default)
    Registrar,
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

    // Handle --update flag first
    if cli.update {
        return run_update().await;
    }

    let mut query = cli.query.ok_or("Query is required")?;

    // Load TLD list for query type detection
    let tld_list = rdap::config::TldList::load().ok();

    // Detect or use specified query type (BEFORE IP normalization)
    let query_type = if let Some(qt) = cli.query_type {
        qt.into()
    } else {
        RdapRequest::detect_type_with_tld_check(&query, |q| {
            tld_list.as_ref().is_some_and(|list| list.is_tld(q))
        })?
    };

    // Normalize IP addresses only if it's an IP query (handles shorthand like 1.1 -> 1.0.0.1)
    if query_type == QueryType::Ip
        && let Some(normalized) = rdap::ip::normalize_ip(&query)
    {
        query = normalized;
    }

    if cli.verbose {
        eprintln!("{} Query: {}", "→".bright_blue(), query.bright_white());
        eprintln!(
            "{} Type:  {}",
            "→".bright_blue(),
            format!("{query_type}").bright_yellow()
        );
    }

    // Build request
    let mut request = RdapRequest::new(query_type, &query);

    if let Some(server_url) = cli.server {
        let url = url::Url::parse(&server_url)?;
        request = request.with_server(url);
        if cli.verbose {
            eprintln!(
                "{} Server: {}",
                "→".bright_blue(),
                server_url.bright_green()
            );
        }
    }

    // Create client
    let client = RdapClient::new()?
        .with_timeout(std::time::Duration::from_secs(cli.timeout))
        .with_follow_referral(!cli.no_referral);

    // Execute query
    if cli.verbose {
        eprintln!("\n{} Querying RDAP server...\n", "⟳".bright_blue());
    }

    // Use query_with_referral to get both registry and registrar data
    let query_result = client.query_with_referral(&request).await?;

    // Display result
    match cli.format {
        OutputFormat::Text => {
            println!(); // Empty line before output
            // For domain queries with registrar data, show both
            if query_result.registrar.is_some() && query_type == QueryType::Domain {
                // Show abuse contact from registrar first (if available)
                if let Some(rdap::RdapObject::Domain(domain)) = &query_result.registrar {
                    rdap::display::display_domain_contacts(domain, &query, false);
                }

                // Show registry server URL and data
                println!("Query from {}", query_result.registry_url.as_str().cyan());
                println!();
                query_result.registry.display(cli.verbose);

                // Show registrar server URL and data
                if let Some(registrar) = &query_result.registrar {
                    println!();
                    if let Some(registrar_url) = &query_result.registrar_url {
                        println!("Query from {}", registrar_url.as_str().green());
                        println!();
                    }
                    registrar.display(cli.verbose);
                }
            } else {
                // Show contacts first based on query type
                match &query_type {
                    QueryType::Tld => {
                        if let rdap::RdapObject::Domain(domain) = &query_result.registry {
                            rdap::display::display_domain_contacts(domain, &query, true);
                        }
                    }
                    QueryType::Domain => {
                        if let rdap::RdapObject::Domain(domain) = &query_result.registry {
                            rdap::display::display_domain_contacts(domain, &query, false);
                        }
                    }
                    QueryType::Ip => {
                        if let rdap::RdapObject::IpNetwork(ip) = &query_result.registry {
                            // For display, use the original query (including CIDR if specified)
                            rdap::display::display_ip_abuse_contact(ip, &query);
                        }
                    }
                    QueryType::Autnum => {
                        let display_query = if query.to_uppercase().starts_with("AS") {
                            query.to_uppercase()
                        } else {
                            format!("AS{query}")
                        };
                        if let rdap::RdapObject::Autnum(asn) = &query_result.registry {
                            rdap::display::display_asn_abuse_contact(asn, &display_query);
                        }
                    }
                    _ => {}
                }

                // Show server URL
                println!("Query from {}", query_result.registry_url.as_str().cyan());
                println!();

                // Display the main data
                query_result.registry.display(cli.verbose);
            }
        }
        OutputFormat::Json => {
            let result = match cli.json_source {
                JsonSource::Registry => &query_result.registry,
                JsonSource::Registrar => query_result
                    .registrar
                    .as_ref()
                    .unwrap_or(&query_result.registry),
            };
            let json = serde_json::to_string(result)?;
            println!("{json}");
        }
        OutputFormat::JsonPretty => {
            let result = match cli.json_source {
                JsonSource::Registry => &query_result.registry,
                JsonSource::Registrar => query_result
                    .registrar
                    .as_ref()
                    .unwrap_or(&query_result.registry),
            };
            let json = serde_json::to_string_pretty(result)?;
            println!("{json}");
        }
    }

    Ok(())
}

async fn run_update() -> Result<(), Box<dyn std::error::Error>> {
    use colored::Colorize;
    use rdap::config;

    println!("{} Updating configuration files...", "→".bright_blue());
    println!("  Source: {}", "https://github.com/xtomcom/rdap".cyan());
    println!();

    let result = config::update_configs().await?;

    // Report config.json status
    if result.config_updated {
        println!("{} config.json updated", "✓".bright_green());
    } else if let Some(err) = result.config_error {
        println!("{} config.json: {}", "✗".bright_red(), err);
    }

    // Report tlds.json status
    if result.tlds_updated {
        println!("{} tlds.json updated", "✓".bright_green());
    } else if let Some(err) = result.tlds_error {
        println!("{} tlds.json: {}", "✗".bright_red(), err);
    }

    // Report tlds.txt status
    if result.tld_list_updated {
        println!("{} tlds.txt updated (IANA TLD list)", "✓".bright_green());
    } else if let Some(err) = result.tld_list_error {
        println!("{} tlds.txt: {}", "✗".bright_red(), err);
    }

    println!();

    // Show config directory
    if let Ok(config_dir) = config::user_config_dir() {
        println!(
            "Config directory: {}",
            config_dir.display().to_string().cyan()
        );
        println!();
        println!("{}", "Note:".bright_yellow().bold());
        println!("  - Your custom settings in *.local.json files are preserved");
        println!("  - Create config.local.json or tlds.local.json for local overrides");
    }

    if result.config_updated || result.tlds_updated || result.tld_list_updated {
        Ok(())
    } else {
        Err("Failed to update any configuration files".into())
    }
}
