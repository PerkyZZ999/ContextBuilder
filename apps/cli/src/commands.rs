//! CLI command definitions, routing, and tracing setup.

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use color_eyre::eyre::{Result, eyre};
use contextbuilder_core::pipeline::{
    AddKbConfig, AddKbResult, ProgressReporter,
};
use contextbuilder_shared::{AppConfig, CrawlConfig, init_config, load_config, validate_api_key};
use indicatif::{ProgressBar, ProgressStyle};
use tracing::info;
use url::Url;

// ---------------------------------------------------------------------------
// CLI structure
// ---------------------------------------------------------------------------

/// ContextBuilder — turn documentation into AI-ready knowledge.
#[derive(Parser)]
#[command(
    name = "contextbuilder",
    version,
    about = "Turn documentation URLs into AI-ready artifacts and portable knowledge bases.",
    long_about = None,
)]
pub(crate) struct Cli {
    /// Log format: text (default) or json.
    #[arg(long, default_value = "text", global = true)]
    pub log_format: LogFormat,

    /// Verbosity level (-v, -vv, -vvv).
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Command,
}

/// Log output format.
#[derive(Clone, Debug, clap::ValueEnum)]
pub(crate) enum LogFormat {
    Text,
    Json,
}

/// Top-level CLI subcommands.
#[derive(Subcommand)]
pub(crate) enum Command {
    /// Add a new documentation source and build its knowledge base.
    Add {
        /// Documentation URL to ingest.
        url: String,

        /// Human-readable name for the KB (defaults to URL hostname).
        #[arg(short, long)]
        name: Option<String>,

        /// Output directory for the KB (defaults to var/kb/<id>).
        #[arg(short, long)]
        out: Option<String>,

        /// Discovery mode: auto, llms-txt, or crawl.
        #[arg(short, long, default_value = "auto")]
        mode: String,
    },

    /// Build or rebuild artifacts for an existing KB.
    Build {
        /// KB path or ID.
        #[arg(long)]
        kb: String,

        /// Artifacts to emit (comma-separated). Defaults to all.
        #[arg(long)]
        emit: Option<String>,
    },

    /// Update an existing KB from upstream changes.
    Update {
        /// KB path or ID.
        #[arg(long)]
        kb: String,

        /// Remove pages no longer present upstream.
        #[arg(long)]
        prune: bool,

        /// Force re-crawl even if content hashes match.
        #[arg(long)]
        force: bool,
    },

    /// List all registered knowledge bases.
    List,

    /// Launch the interactive TUI.
    Tui,

    /// Start the MCP server.
    #[command(name = "mcp")]
    Mcp {
        /// Subcommand for MCP operations.
        #[command(subcommand)]
        action: McpAction,
    },

    /// Configuration management.
    Config {
        /// Config subcommand.
        #[command(subcommand)]
        action: ConfigAction,
    },
}

/// MCP server subcommands.
#[derive(Subcommand)]
pub(crate) enum McpAction {
    /// Start the MCP server.
    Serve {
        /// KB path(s) to serve (can be specified multiple times).
        #[arg(long)]
        kb: Vec<String>,

        /// Root directory to discover KBs from (e.g., var/kb).
        #[arg(long)]
        kb_root: Option<String>,

        /// Transport: stdio or http.
        #[arg(long, default_value = "stdio")]
        transport: String,

        /// Port for HTTP transport.
        #[arg(long, default_value = "3100")]
        port: u16,
    },
    /// Print MCP client configuration snippets.
    Config {
        /// Target client: vscode, claude-desktop, or cursor.
        #[arg(long, default_value = "vscode")]
        target: String,

        /// KB path(s) to include in config.
        #[arg(long)]
        kb: Vec<String>,

        /// KB root directory to include in config.
        #[arg(long)]
        kb_root: Option<String>,
    },
}

/// Config subcommands.
#[derive(Subcommand)]
pub(crate) enum ConfigAction {
    /// Initialize config file with defaults.
    Init,
    /// Show resolved configuration.
    Show,
}

// ---------------------------------------------------------------------------
// Tracing setup
// ---------------------------------------------------------------------------

/// Initialize tracing based on CLI flags.
pub(crate) fn init_tracing(cli: &Cli) {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = match cli.verbose {
        0 => "contextbuilder=info",
        1 => "contextbuilder=debug",
        _ => "contextbuilder=trace",
    };

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(filter));

    match cli.log_format {
        LogFormat::Text => {
            fmt()
                .with_env_filter(env_filter)
                .with_target(false)
                .init();
        }
        LogFormat::Json => {
            fmt()
                .json()
                .with_env_filter(env_filter)
                .init();
        }
    }
}

// ---------------------------------------------------------------------------
// Command dispatch
// ---------------------------------------------------------------------------

/// Run the CLI command.
pub(crate) async fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Add {
            url,
            name,
            out,
            mode,
        } => cmd_add(&url, name.as_deref(), out.as_deref(), &mode).await,
        Command::Build { kb, emit } => cmd_build(&kb, emit.as_deref()).await,
        Command::Update { kb, prune, force } => cmd_update(&kb, prune, force).await,
        Command::List => cmd_list().await,
        Command::Tui => cmd_tui().await,
        Command::Mcp { action } => match action {
            McpAction::Serve {
                kb,
                kb_root,
                transport,
                port,
            } => cmd_mcp_serve(&kb, kb_root.as_deref(), &transport, port).await,
            McpAction::Config {
                target,
                kb,
                kb_root,
            } => cmd_mcp_config(&target, &kb, kb_root.as_deref()).await,
        },
        Command::Config { action } => match action {
            ConfigAction::Init => cmd_config_init().await,
            ConfigAction::Show => cmd_config_show().await,
        },
    }
}

// ---------------------------------------------------------------------------
// Placeholder command handlers
// ---------------------------------------------------------------------------

async fn cmd_add(url: &str, name: Option<&str>, out: Option<&str>, mode: &str) -> Result<()> {
    // Validate API key before doing anything
    let config = load_config()?;
    validate_api_key(&config)?;

    // Parse URL
    let parsed_url = Url::parse(url)
        .map_err(|e| eyre!("invalid URL '{url}': {e}"))?;

    // Derive name from hostname if not provided
    let kb_name = name.map(String::from).unwrap_or_else(|| {
        parsed_url
            .host_str()
            .unwrap_or("unknown")
            .to_string()
    });

    // Determine output root
    let cwd = std::env::current_dir()
        .map_err(|e| eyre!("cannot determine working directory: {e}"))?;

    let output_root = match out {
        Some(p) => PathBuf::from(p),
        None => {
            // Default to <workspace>/var/kb/
            cwd.join("var").join("kb")
        }
    };

    // Build crawl config from loaded config
    let crawl_config = CrawlConfig::from(&config);

    let add_config = AddKbConfig {
        url: parsed_url,
        name: kb_name.clone(),
        output_root,
        mode: mode.to_string(),
        crawl: crawl_config,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        model_id: config.openrouter.default_model.clone(),
        bridge_cmd: "bun".to_string(),
        bridge_script: "packages/ts/openrouter-provider/src/bridge.ts".to_string(),
        bridge_working_dir: cwd.to_string_lossy().to_string(),
    };

    info!(
        url,
        name = %kb_name,
        mode,
        "adding documentation source"
    );

    // Set up progress reporting
    let reporter = CliProgress::new();

    let result = contextbuilder_core::pipeline::add_kb(&add_config, &reporter).await?;

    // Print summary
    println!();
    println!("  Knowledge base created successfully!");
    println!("  ID:     {}", result.kb_id);
    println!("  Name:   {kb_name}");
    println!("  Pages:  {}", result.page_count);
    println!("  Method: {}", result.method);
    println!("  Path:   {}", result.kb_path.display());
    println!(
        "  Time:   {:.1}s",
        result.elapsed.as_secs_f64()
    );
    println!();

    Ok(())
}

// ---------------------------------------------------------------------------
// CLI progress reporter
// ---------------------------------------------------------------------------

/// CLI progress reporter using indicatif spinners/bars.
struct CliProgress {
    spinner: ProgressBar,
}

impl CliProgress {
    fn new() -> Self {
        let spinner = ProgressBar::new_spinner();
        spinner.set_style(
            ProgressStyle::with_template("{spinner:.cyan} {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        spinner.enable_steady_tick(std::time::Duration::from_millis(80));
        Self { spinner }
    }
}

impl ProgressReporter for CliProgress {
    fn phase(&self, name: &str) {
        self.spinner.set_message(name.to_string());
    }

    fn page_fetched(&self, url: &str, current: usize, total_estimate: usize) {
        self.spinner.set_message(format!(
            "Fetching [{current}/{total_estimate}] {url}"
        ));
    }

    fn page_converted(&self, path: &str, current: usize, total: usize) {
        self.spinner.set_message(format!(
            "Converting [{current}/{total}] {path}"
        ));
    }

    fn done(&self, _result: &AddKbResult) {
        self.spinner.finish_and_clear();
    }
}

async fn cmd_build(kb: &str, emit: Option<&str>) -> Result<()> {
    info!(kb, emit = emit.unwrap_or("all"), "building artifacts");
    println!("build: not yet implemented (kb={kb})");
    Ok(())
}

async fn cmd_update(kb: &str, prune: bool, force: bool) -> Result<()> {
    let config = load_config()?;
    validate_api_key(&config)?;

    let kb_path = PathBuf::from(kb);
    if !kb_path.join("manifest.json").exists() {
        return Err(eyre!("no manifest.json found at '{kb}' — is this a valid KB directory?"));
    }

    let crawl_config = CrawlConfig::from(&config);

    let update_config = contextbuilder_core::update::UpdateKbConfig {
        kb_path,
        crawl: crawl_config,
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        prune,
        force,
    };

    info!(kb, prune, force, "updating knowledge base");

    let reporter = CliProgress::new();
    let result = contextbuilder_core::update::update_kb(&update_config, &reporter).await?;

    println!();
    println!("  Knowledge base updated!");
    println!("  ID:        {}", result.kb_id);
    println!("  Added:     {}", result.pages_added);
    println!("  Changed:   {}", result.pages_changed);
    println!("  Unchanged: {}", result.pages_unchanged);
    println!("  Removed:   {}", result.pages_removed);
    println!("  Total:     {}", result.page_count);
    println!(
        "  Time:      {:.1}s",
        result.elapsed.as_secs_f64()
    );
    println!();

    Ok(())
}

async fn cmd_list() -> Result<()> {
    info!("listing knowledge bases");
    println!("list: not yet implemented");
    Ok(())
}

async fn cmd_tui() -> Result<()> {
    info!("launching TUI");
    println!("tui: not yet implemented");
    Ok(())
}

async fn cmd_mcp_serve(
    kbs: &[String],
    kb_root: Option<&str>,
    transport: &str,
    port: u16,
) -> Result<()> {
    // Validate transport
    if transport != "stdio" && transport != "http" {
        return Err(eyre!(
            "invalid transport '{transport}': expected 'stdio' or 'http'"
        ));
    }

    // Check that bun is available
    let bun_check = std::process::Command::new("bun")
        .arg("--version")
        .output();

    match bun_check {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            info!(bun_version = %version.trim(), "bun runtime found");
        }
        _ => {
            return Err(eyre!(
                "bun runtime not found. Install Bun: https://bun.sh/docs/installation"
            ));
        }
    }

    // Resolve the MCP server script path relative to the CLI binary's location
    // or the current working directory
    let cwd = std::env::current_dir()?;
    let server_script = cwd.join("apps/mcp-server/src/index.ts");

    if !server_script.exists() {
        return Err(eyre!(
            "MCP server script not found at '{}'. Run from the project root or install the package.",
            server_script.display()
        ));
    }

    // Build args for the subprocess
    let mut args: Vec<String> = vec![
        "run".to_string(),
        server_script.to_string_lossy().to_string(),
    ];

    for kb_path in kbs {
        // Validate KB path
        let p = PathBuf::from(kb_path);
        if !p.join("manifest.json").exists() {
            return Err(eyre!(
                "no manifest.json found at '{kb_path}' — is this a valid KB directory?"
            ));
        }
        args.push("--kb".to_string());
        args.push(
            std::fs::canonicalize(&p)?
                .to_string_lossy()
                .to_string(),
        );
    }

    if let Some(root) = kb_root {
        let p = PathBuf::from(root);
        if !p.is_dir() {
            return Err(eyre!("KB root '{root}' is not a directory"));
        }
        args.push("--kb-root".to_string());
        args.push(
            std::fs::canonicalize(&p)?
                .to_string_lossy()
                .to_string(),
        );
    }

    args.push("--transport".to_string());
    args.push(transport.to_string());

    if transport == "http" {
        args.push("--port".to_string());
        args.push(port.to_string());
    }

    info!(
        transport,
        port,
        kb_count = kbs.len(),
        "starting MCP server subprocess"
    );

    if transport == "http" {
        println!("Starting MCP server on http://localhost:{port}/mcp");
    }

    // Spawn bun subprocess
    let mut child = std::process::Command::new("bun")
        .args(&args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .map_err(|e| eyre!("failed to spawn bun: {e}"))?;

    // Wait for the child to finish (ctrl-C forwarded via signal inheritance)
    let status = child
        .wait()
        .map_err(|e| eyre!("failed to wait for MCP server: {e}"))?;

    if !status.success() {
        return Err(eyre!(
            "MCP server exited with status: {}",
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

async fn cmd_mcp_config(
    target: &str,
    kbs: &[String],
    kb_root: Option<&str>,
) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let binary_path = cwd.join("apps/mcp-server/src/index.ts");

    // Build the args list
    let mut kb_args: Vec<serde_json::Value> = Vec::new();
    for kb_path in kbs {
        let abs = std::fs::canonicalize(kb_path)
            .unwrap_or_else(|_| PathBuf::from(kb_path));
        kb_args.push(serde_json::Value::String("--kb".to_string()));
        kb_args.push(serde_json::Value::String(
            abs.to_string_lossy().to_string(),
        ));
    }
    if let Some(root) = kb_root {
        let abs = std::fs::canonicalize(root)
            .unwrap_or_else(|_| PathBuf::from(root));
        kb_args.push(serde_json::Value::String("--kb-root".to_string()));
        kb_args.push(serde_json::Value::String(
            abs.to_string_lossy().to_string(),
        ));
    }

    let mut run_args = vec![
        serde_json::Value::String("run".to_string()),
        serde_json::Value::String(binary_path.to_string_lossy().to_string()),
    ];
    run_args.extend(kb_args);

    match target {
        "vscode" => {
            let config = serde_json::json!({
                "servers": {
                    "contextbuilder": {
                        "type": "stdio",
                        "command": "bun",
                        "args": run_args,
                    }
                }
            });
            println!("// .vscode/mcp.json");
            println!(
                "{}",
                serde_json::to_string_pretty(&config)?
            );
        }
        "claude-desktop" => {
            let config = serde_json::json!({
                "mcpServers": {
                    "contextbuilder": {
                        "command": "bun",
                        "args": run_args,
                    }
                }
            });
            println!("// claude_desktop_config.json");
            println!(
                "{}",
                serde_json::to_string_pretty(&config)?
            );
        }
        "cursor" => {
            let config = serde_json::json!({
                "mcpServers": {
                    "contextbuilder": {
                        "command": "bun",
                        "args": run_args,
                    }
                }
            });
            println!("// Cursor MCP settings");
            println!(
                "{}",
                serde_json::to_string_pretty(&config)?
            );
        }
        _ => {
            return Err(eyre!(
                "unknown config target '{target}': expected 'vscode', 'claude-desktop', or 'cursor'"
            ));
        }
    }

    Ok(())
}

async fn cmd_config_init() -> Result<()> {
    let path = init_config()?;
    println!("Config initialized at: {}", path.display());
    Ok(())
}

async fn cmd_config_show() -> Result<()> {
    let config: AppConfig = load_config()?;
    let toml_str = toml::to_string_pretty(&config)?;
    println!("{toml_str}");
    Ok(())
}
