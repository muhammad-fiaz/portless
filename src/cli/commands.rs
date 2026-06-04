//! Command implementations.

use crate::cli::opts::{GlobalOpts, ProxyStartOpts};
use crate::common::net;
use crate::common::{Error, Result};
use crate::config::{Env, PortlessConfig};
use crate::discovery::Project;
use crate::hosts::{self, HostsLine};
use crate::platform::Paths;
use crate::proxy::server::{ProxyConfig, ProxyServer};
use crate::routing::hostname::{Host, sanitize_label};
use crate::routing::match_::Route;
use crate::routing::tld::Tld;
use crate::service::ServiceManager;
use crate::state::Registry;
use crate::state::proxy_state::ProxyState;
use crate::worktree::Worktree;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

/// Top-level CLI definition.
#[derive(Debug, Parser)]
#[command(
    name = "portless",
    version,
    about = "Replace port numbers with stable, named .localhost URLs for local development.",
    long_about = "Portless runs a reverse proxy and registers stable hostnames for your dev servers.\n\nRun `portless help` or visit https://muhammad-fiaz.github.io/portless for documentation."
)]
pub struct Cli {
    /// Global options (verbosity, state dir).
    #[command(flatten)]
    pub global: GlobalOpts,
    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Option<CommandKind>,
}

/// All subcommands.
#[derive(Debug, Subcommand)]
pub enum CommandKind {
    /// Run a command through the proxy (with inferred name).
    #[command(trailing_var_arg = true, allow_hyphen_values = true)]
    Run {
        /// Override the inferred name.
        #[arg(long)]
        name: Option<String>,
        /// Override the script to run.
        #[arg(long, default_value = "dev")]
        script: String,
        /// Disable worktree prefix.
        #[arg(long)]
        no_worktree: bool,
        /// Tailnet sharing.
        #[arg(long, env = "PORTLESS_TAILSCALE")]
        tailscale: bool,
        /// Funnel sharing.
        #[arg(long, env = "PORTLESS_FUNNEL")]
        funnel: bool,
        /// The command and arguments to run.
        #[arg(allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
    /// Run a named app through the proxy: `portless <name> <command...>`.
    #[command(trailing_var_arg = true, allow_hyphen_values = true)]
    Named {
        /// The app name (e.g. `myapp`, `api.myapp`).
        name: String,
        /// Force the route to be reassigned.
        #[arg(long)]
        force: bool,
        /// Use a fixed port.
        #[arg(long, env = "PORTLESS_APP_PORT")]
        app_port: Option<u16>,
        /// Share on Tailscale.
        #[arg(long, env = "PORTLESS_TAILSCALE")]
        tailscale: bool,
        /// Share on Tailscale Funnel.
        #[arg(long, env = "PORTLESS_FUNNEL")]
        funnel: bool,
        /// The command and arguments.
        #[arg(allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
    /// Print the URL for a service.
    Get {
        /// Service name.
        name: String,
        /// Disable worktree prefix.
        #[arg(long)]
        no_worktree: bool,
    },
    /// Register a static alias (e.g. for a Docker service).
    Alias {
        /// The name to register.
        name: String,
        /// The TCP port the service listens on.
        port: Option<u16>,
        /// Overwrite an existing alias.
        #[arg(long)]
        force: bool,
        /// Remove the alias instead of adding it.
        #[arg(long, conflicts_with_all = ["port", "force"])]
        remove: bool,
    },
    /// List active routes.
    List,
    /// Trust the local CA in the system trust store.
    Trust,
    /// Remove the local CA from the system trust store.
    Untrust,
    /// Remove state, CA trust entry, and hosts block.
    Clean,
    /// Kill orphaned dev servers from crashed sessions.
    Prune {
        /// Force kill (SIGKILL) instead of polite (SIGTERM).
        #[arg(long)]
        force: bool,
    },
    /// Manage /etc/hosts synchronization.
    Hosts {
        /// Which hosts action to perform.
        #[command(subcommand)]
        action: HostsAction,
    },
    /// Proxy control.
    Proxy {
        /// Which proxy action to perform.
        #[command(subcommand)]
        action: ProxyAction,
    },
    /// Install/uninstall/status the OS startup service.
    Service {
        /// Which service action to perform.
        #[command(subcommand)]
        action: ServiceAction,
    },
    /// Print version information.
    Version,
    /// Catch `portless <name> <cmd...>` (shorthand for `portless named <name> <cmd...>`).
    ///
    /// This allows the TypeScript-compatible syntax:
    ///   portless myapp next dev
    ///   portless api.myapp pnpm start
    #[command(external_subcommand)]
    Positional(Vec<String>),
}

/// /etc/hosts subcommands.
#[derive(Debug, Subcommand)]
pub enum HostsAction {
    /// Add current routes to /etc/hosts.
    Sync,
    /// Remove portless entries from /etc/hosts.
    Clean,
}

/// Proxy subcommands.
#[derive(Debug, Subcommand)]
pub enum ProxyAction {
    /// Start the proxy.
    Start(ProxyStartOpts),
    /// Stop the proxy.
    Stop,
    /// Print the proxy status.
    Status,
}

/// Service subcommands.
#[derive(Debug, Subcommand)]
pub enum ServiceAction {
    /// Install the service.
    Install(ProxyStartOpts),
    /// Uninstall the service.
    Uninstall,
    /// Print the service status.
    Status,
}



/// Dispatch a parsed `Cli`.
pub async fn run(cli: Cli) -> Result<()> {
    init_tracing(cli.global.verbose);
    // PORTLESS=0 -> run the command directly.
    if let Some(v) = &cli.global.bypass
        && v == "0"
    {
        return Err(Error::Config(
            "PORTLESS=0 detected: cannot be set in the global flag path; use `portless run --bypass` (not implemented) or unset the env var"
                .into(),
        ));
    }
    let paths = match &cli.global.state_dir {
        Some(p) => Paths::open(p)?,
        None => Paths::open_default()?,
    };
    match cli.command {
        Some(CommandKind::Run {
            name,
            script,
            no_worktree,
            tailscale,
            funnel,
            cmd,
        }) => cmd_run(name, script, no_worktree, tailscale, funnel, cmd, paths).await,
        Some(CommandKind::Named {
            name,
            force,
            app_port,
            tailscale,
            funnel,
            cmd,
        }) => cmd_named(name, force, app_port, tailscale, funnel, cmd, paths).await,
        Some(CommandKind::Get { name, no_worktree }) => cmd_get(name, no_worktree, paths).await,
        Some(CommandKind::Alias {
            name,
            port,
            force,
            remove,
        }) => cmd_alias(name, port, force, remove, paths).await,
        Some(CommandKind::List) => cmd_list(paths).await,
        Some(CommandKind::Trust) => cmd_trust(paths).await,
        Some(CommandKind::Untrust) => cmd_untrust(paths).await,
        Some(CommandKind::Clean) => cmd_clean(paths).await,
        Some(CommandKind::Prune { force }) => cmd_prune(force, paths).await,
        Some(CommandKind::Hosts { action }) => match action {
            HostsAction::Sync => cmd_hosts_sync(paths).await,
            HostsAction::Clean => cmd_hosts_clean().await,
        },
        Some(CommandKind::Proxy { action }) => match action {
            ProxyAction::Start(opts) => cmd_proxy_start(opts, paths).await,
            ProxyAction::Stop => cmd_proxy_stop(paths).await,
            ProxyAction::Status => cmd_proxy_status(paths).await,
        },
        Some(CommandKind::Service { action }) => match action {
            ServiceAction::Install(opts) => cmd_service_install(opts, paths).await,
            ServiceAction::Uninstall => cmd_service_uninstall().await,
            ServiceAction::Status => cmd_service_status().await,
        },
        Some(CommandKind::Version) => {
            println!("portless {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }

        Some(CommandKind::Positional(args)) => {
            // `portless <name> [cmd...]` — treat the first positional as the
            // app name and the rest as the command to run.
            if args.is_empty() {
                return Err(Error::Config("missing app name".into()));
            }
            let name = args[0].clone();
            let cmd = args[1..].to_vec();
            cmd_named(name, false, None, false, false, cmd, paths).await
        }
        None => cmd_zero_arg(paths).await,
    }
}

fn init_tracing(verbosity: u8) {
    use tracing_subscriber::{EnvFilter, fmt};
    let default_level = match verbosity {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_level));
    let _ = fmt().with_env_filter(filter).try_init();
}

async fn cmd_zero_arg(paths: Paths) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let cfg = PortlessConfig::load(&cwd).await?;
    let project = Project::discover(&cwd).await?;

    // Detect whether the current directory has any runnable project files.
    // "Runnable" means: a package.json with scripts, an explicit portless.json
    // configuration, or any recognised language ecosystem file (Cargo.toml,
    // go.mod, pyproject.toml, etc.).
    let has_js_script = !project.scripts.is_empty();
    let has_portless_config = cfg.name.is_some() || cfg.script.is_some();
    let has_any_framework = project.framework.is_some();

    if !has_js_script && !has_portless_config && !has_any_framework {
        // No recognisable project in this directory — print a helpful message.
        eprintln!(
            "portless: no runnable project detected in '{}'.",
            cwd.display()
        );
        eprintln!();
        eprintln!("portless works with any language or runtime. Pass the command explicitly:");
        eprintln!();
        eprintln!("  # Rust");
        eprintln!("  portless run cargo run");
        eprintln!();
        eprintln!("  # Go");
        eprintln!("  portless run go run .");
        eprintln!();
        eprintln!("  # Python");
        eprintln!("  portless run uv run main.py");
        eprintln!("  portless run python main.py");
        eprintln!();
        eprintln!("  # Node.js / npm");
        eprintln!("  portless run npm run dev");
        eprintln!("  portless run npx next dev");
        eprintln!();
        eprintln!("  # Node.js / pnpm / yarn / bun");
        eprintln!("  portless run pnpm dev");
        eprintln!("  portless run yarn dev");
        eprintln!("  portless run bun dev");
        eprintln!();
        eprintln!("  # Deno");
        eprintln!("  portless run deno task dev");
        eprintln!();
        eprintln!("  # Named app (any command)");
        eprintln!("  portless myapp cargo run --release");
        eprintln!("  portless api python -m uvicorn main:app --reload");
        eprintln!();
        eprintln!("Run `portless --help` for full usage.");
        std::process::exit(0);
    }

    if project.is_monorepo() {
        run_monorepo(&cfg, &project, &cwd, &paths).await
    } else {
        let script = cfg.effective_script();
        // For JS projects, verify the script actually exists before spawning.
        if has_js_script && !project.has_script(&script) && !has_portless_config {
            eprintln!("portless: no '{script}' script found in package.json.");
            let available = project.scripts.keys().cloned().collect::<Vec<_>>().join(", ");
            if !available.is_empty() {
                eprintln!("  Available scripts: {available}");
            }
            eprintln!();
            eprintln!("Tip: use `portless run <command>` to run any command directly.");
            std::process::exit(1);
        }
        cmd_run(None, script, false, false, false, vec![], paths).await
    }
}


async fn run_monorepo(
    _cfg: &PortlessConfig,
    project: &Project,
    cwd: &std::path::Path,
    _paths: &Paths,
) -> Result<()> {
    use crate::discovery::monorepo::Workspace;
    let kind = match &project.kind {
        crate::discovery::project::ProjectKind::Monorepo(k) => *k,
        _ => return Err(Error::Config("not a monorepo".into())),
    };
    let workspaces = Workspace::discover(cwd, kind).await?;
    let mut started = 0usize;
    for ws in &workspaces {
        if ws.scripts.is_empty() || !ws.scripts.contains_key("dev") {
            continue;
        }
        println!(
            "starting {} ({})",
            ws.rel_path,
            ws.name.clone().unwrap_or_default()
        );
        started += 1;
    }
    if started == 0 {
        eprintln!("portless: no workspace packages with a 'dev' script found.");
        eprintln!("  Run `portless run <command>` from an individual package directory.");
        std::process::exit(1);
    }
    Ok(())
}

/// Check if the proxy is currently running and alive.
async fn is_proxy_alive(paths: &Paths) -> bool {
    if let Ok(pid_str) = tokio::fs::read_to_string(paths.proxy_pid()).await
        && let Ok(pid) = pid_str.trim().parse::<u32>()
    {
        return crate::process::pid_is_alive(pid);
    }
    false
}

/// Ensure the proxy is running, auto-starting it in the background if needed.
async fn ensure_proxy_running(paths: &Paths) -> Result<()> {
    if is_proxy_alive(paths).await {
        return Ok(());
    }

    let env = Env::load();
    let store = crate::state::proxy_state::Store::open(paths.clone()).await.ok();
    let state = if let Some(ref s) = store {
        s.snapshot().await
    } else {
        crate::state::proxy_state::ProxyState::default()
    };

    let port = env.port().unwrap_or_else(|| {
        if state.port != 0 {
            state.port
        } else if env.https_disabled() {
            80
        } else {
            443
        }
    });

    let tld = env.tld().map(|s| s.to_string()).unwrap_or_else(|| {
        if !state.tld.as_str().is_empty() {
            state.tld.as_str().to_string()
        } else {
            "localhost".to_string()
        }
    });

    let https = if env.https_disabled() {
        false
    } else if env.https_forced() {
        true
    } else if state.port != 0 {
        state.https
    } else {
        true
    };

    let exe = std::env::current_exe()?;
    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("proxy").arg("start");

    if !https {
        cmd.arg("--no-tls");
    }

    cmd.arg("--port").arg(port.to_string());
    cmd.arg("--tld").arg(&tld);
    cmd.arg("--foreground");

    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    cmd.stdin(std::process::Stdio::null());

    println!("portless: proxy not running. Auto-starting proxy on port {}...", port);
    cmd.spawn()?;

    for _ in 0..10 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        if is_proxy_alive(paths).await {
            break;
        }
    }

    Ok(())
}

/// Build the proxy-aware environment for a child process.
fn build_child_env(
    full_host: &str,
    port: u16,
    tld: &str,
    tailscale_url: Option<&str>,
    https: bool,
) -> Vec<(String, String)> {
    let mut env: Vec<(String, String)> = vec![
        ("PORT".to_string(), port.to_string()),
        ("HOST".to_string(), "127.0.0.1".to_string()),
        (
            "PORTLESS_URL".to_string(),
            format!("{}://{}", if https { "https" } else { "http" }, full_host),
        ),
        ("PORTLESS_TLD".to_string(), tld.to_string()),
        (
            "PORTLESS_HTTPS".to_string(),
            if https { "1".to_string() } else { "0".to_string() },
        ),
    ];
    if let Some(t) = tailscale_url {
        env.push(("PORTLESS_TAILSCALE_URL".to_string(), t.to_string()));
    }
    env
}

/// Resolve the URL to pass to the child process.
fn child_url(full_host: &str, https: bool) -> String {
    if https {
        format!("https://{full_host}")
    } else {
        format!("http://{full_host}")
    }
}

fn is_framework_needing_port(name: &str) -> bool {
    crate::discovery::framework::Framework::from_basename(name)
        .map(|fw| !fw.port_flags().is_empty())
        .unwrap_or(false)
}

fn get_package_runners() -> std::collections::HashMap<&'static str, Vec<&'static str>> {
    let mut m = std::collections::HashMap::new();
    m.insert("npx", vec![]);
    m.insert("bunx", vec![]);
    m.insert("pnpx", vec![]);
    m.insert("yarn", vec!["dlx", "exec"]);
    m.insert("pnpm", vec!["dlx", "exec"]);
    m
}

fn find_framework_basename(command_args: &[String]) -> Option<String> {
    if command_args.is_empty() {
        return None;
    }

    let runners = get_package_runners();

    let first_arg = std::path::Path::new(&command_args[0])
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&command_args[0]);

    if is_framework_needing_port(first_arg) {
        return Some(first_arg.to_string());
    }

    if let Some(subcommands) = runners.get(first_arg) {
        let mut i = 1;
        if !subcommands.is_empty() {
            // Skip flags before the subcommand (e.g. yarn --foo dlx)
            while i < command_args.len() && command_args[i].starts_with('-') {
                i += 1;
            }
            if i >= command_args.len() {
                return None;
            }
            if !subcommands.contains(&command_args[i].as_str()) {
                // Not a recognized subcommand — might be an implicit bin (e.g. `yarn vite`)
                let name = std::path::Path::new(&command_args[i])
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or(&command_args[i]);
                if is_framework_needing_port(name) {
                    return Some(name.to_string());
                }
                return None;
            }
            i += 1;
        }

        // Skip runner flags (e.g. `--bun`, `--yes`)
        while i < command_args.len() && command_args[i].starts_with('-') {
            i += 1;
        }

        if i >= command_args.len() {
            return None;
        }
        let name = std::path::Path::new(&command_args[i])
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&command_args[i]);
        if is_framework_needing_port(name) {
            return Some(name.to_string());
        }
    }

    None
}

/// Build the spawn command, appending the right `--port` / `--host` flags for
/// known frameworks that ignore `PORT`.
fn build_command_for(cmd: &[String], port: u16, lan_mode: bool) -> Vec<String> {
    if cmd.is_empty() {
        return vec![];
    }
    let mut out = cmd.to_vec();
    if let Some(basename) = find_framework_basename(cmd)
        && let Some(fw) = crate::discovery::framework::Framework::from_basename(&basename)
    {
        if !out.iter().any(|a| a == "--port" || a == "-p") {
            out.push("--port".to_string());
            out.push(port.to_string());
            if fw.strict_port() {
                out.push("--strictPort".to_string());
            }
        }
        if !out.iter().any(|a| a == "--host" || a == "-H") {
            let is_expo_lan = fw == crate::discovery::framework::Framework::Expo && lan_mode;
            if !is_expo_lan {
                let host_val = if fw == crate::discovery::framework::Framework::Expo { "localhost" } else { "127.0.0.1" };
                out.push("--host".to_string());
                out.push(host_val.to_string());
            }
        }
    }
    out
}

/// Register a route, spawn a child, wait for exit, unregister.
#[allow(clippy::too_many_arguments)]
async fn register_spawn_wait(
    full_host: String,
    port: u16,
    program: String,
    args: Vec<String>,
    cwd: std::path::PathBuf,
    env: Vec<(String, String)>,
    force: bool,
    paths: Paths,
) -> Result<i32> {
    let registry = Registry::open(paths.clone()).await?;
    if let Some(existing) = registry.get(&full_host) {
        if !force {
            return Err(Error::RouteExists(full_host.clone()));
        }
        // --force: kill the existing process first.
        if existing.pid != 0 && crate::process::pid_is_alive(existing.pid) {
            let _ = crate::process::kill_process(existing.pid);
        }
        let _ = registry.remove(&full_host).await;
    }
    // Persist a placeholder route so the proxy / `portless list` see it.
    let placeholder = Route::new(&full_host, port, 0, Some(program.clone()), args.clone());
    registry.insert(placeholder, force).await?;

    // Spawn the child with inherited stdio so the developer sees live output.
    // Create the logs directory best-effort (future tee support).
    let logs_dir = paths.logs_dir();
    let _ = tokio::fs::create_dir_all(&logs_dir).await;

    let mut spawner = crate::process::spawn::Spawner::new(&program)
        .args(&args)
        .cwd(cwd.clone())
        .envs(env.clone())
        .inherit_stdio(true);

    if Env::load().tty_required() {
        spawner = spawner.force_color(true);
    }

    let mut child = match spawner.spawn().await {
        Ok(c) => c,
        Err(e) => {
            // Roll back the placeholder.
            let _ = registry.remove(&full_host).await;
            return Err(e);
        }
    };
    let pid = child.pid().unwrap_or(0);
    // Update the route with the real PID.
    let updated = Route::new(&full_host, port, pid, Some(program.clone()), args.clone());
    registry.remove(&full_host).await?;
    registry.insert(updated, true).await?;
    println!("https://{full_host} -> http://127.0.0.1:{port} (pid {pid})");
    // Wait for the child to exit.
    let exit = child.wait().await?;
    // Unregister.
    let _ = registry.remove(&full_host).await;
    let code = exit
        .status
        .and_then(|s| s.code())
        .or(exit.signal)
        .unwrap_or(1);
    Ok(code)
}


async fn cmd_run(
    name: Option<String>,
    script: String,
    no_worktree: bool,
    tailscale: bool,
    funnel: bool,
    cmd: Vec<String>,
    paths: Paths,
) -> Result<()> {
    ensure_proxy_running(&paths).await?;
    let store = crate::state::proxy_state::Store::open(paths.clone()).await?;
    let state = store.snapshot().await;
    let tld = state.tld.clone();
    let lan_mode = state.lan;
    let https = state.https;

    let cwd = std::env::current_dir()?;
    let cfg = PortlessConfig::load(&cwd).await?;
    let base_name = match name {
        Some(n) => sanitize_label(&n),
        None => sanitize_label(
            &cfg.effective_name(&cwd, &crate::discovery::project::ProjectKind::Single)
                .await,
        ),
    };
    let worktree_prefix = if no_worktree {
        None
    } else {
        Worktree::detect(&cwd)
            .await
            .ok()
            .flatten()
            .and_then(|w| w.hostname_prefix())
    };
    let full_host = compose_hostname(base_name, worktree_prefix, &tld);
    let _ = Host::new(&full_host)?;
    // Pick a port and the actual command.
    let (program, args) = if cmd.is_empty() {
        // Resolve "npm run <script>" (or pnpm/yarn/bun equivalent).
        let pm = crate::discovery::package_manager::PackageManager::detect(&cwd).await?;
        let runner = pm.runner();
        (runner.to_string(), vec!["run".to_string(), script.clone()])
    } else {
        (cmd[0].clone(), cmd[1..].to_vec())
    };
    let app_port = Env::load()
        .app_port()
        .or(cfg.app_port)
        .unwrap_or_else(|| net::find_free_port(4000, 5000).unwrap_or(4000));
    let tailscale_url = if tailscale || funnel {
        match crate::tailscale::Tailscale::new() {
            Ok(ts) => ts.serve_url(&full_host).ok().flatten(),
            Err(_) => None,
        }
    } else {
        None
    };
    let env = build_child_env(&full_host, app_port, tld.as_str(), tailscale_url.as_deref(), https);
    
    let mut command_line = vec![program.clone()];
    command_line.extend(args.iter().cloned());
    let final_command_line = build_command_for(&command_line, app_port, lan_mode);
    let final_program = final_command_line[0].clone();
    let final_args = final_command_line[1..].to_vec();

    let code = register_spawn_wait(
        full_host,
        app_port,
        final_program,
        final_args,
        cwd,
        env,
        Env::load().force(),
        paths,
    )
    .await?;
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

async fn cmd_named(
    name: String,
    force: bool,
    app_port: Option<u16>,
    tailscale: bool,
    funnel: bool,
    cmd: Vec<String>,
    paths: Paths,
) -> Result<()> {
    if cmd.is_empty() {
        return Err(Error::Config("missing command".into()));
    }
    // Bypass support: `PORTLESS=0` runs the command directly.
    if std::env::var("PORTLESS").ok().as_deref() == Some("0") {
        let status = std::process::Command::new(&cmd[0])
            .args(&cmd[1..])
            .status()?;
        if !status.success() {
            std::process::exit(status.code().unwrap_or(1));
        }
        return Ok(());
    }

    ensure_proxy_running(&paths).await?;
    let store = crate::state::proxy_state::Store::open(paths.clone()).await?;
    let state = store.snapshot().await;
    let tld = state.tld.clone();
    let lan_mode = state.lan;
    let https = state.https;

    let cwd = std::env::current_dir()?;
    let worktree_prefix = Worktree::detect(&cwd)
        .await
        .ok()
        .flatten()
        .and_then(|w| w.hostname_prefix());
    let base = sanitize_label(&name);
    let full_host = compose_hostname(base, worktree_prefix, &tld);
    let _ = Host::new(&full_host)?;
    let port = match app_port {
        Some(p) => p,
        None => net::find_free_port(4000, 5000)?,
    };
    let tailscale_url = if tailscale || funnel {
        match crate::tailscale::Tailscale::new() {
            Ok(ts) => ts.serve_url(&full_host).ok().flatten(),
            Err(_) => None,
        }
    } else {
        None
    };
    let env = build_child_env(&full_host, port, tld.as_str(), tailscale_url.as_deref(), https);
    
    let final_command_line = build_command_for(&cmd, port, lan_mode);
    let final_program = final_command_line[0].clone();
    let final_args = final_command_line[1..].to_vec();

    let code =
        register_spawn_wait(full_host, port, final_program, final_args, cwd, env, force, paths).await?;
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}

async fn cmd_get(name: String, no_worktree: bool, paths: Paths) -> Result<()> {
    let cwd = std::env::current_dir()?;
    
    let store = crate::state::proxy_state::Store::open(paths.clone()).await.ok();
    let (tld, https) = if let Some(store) = store {
        let snapshot = store.snapshot().await;
        if !snapshot.tld.as_str().is_empty() {
            (snapshot.tld.clone(), snapshot.https)
        } else {
            (Tld::new(Env::load().tld().unwrap_or("localhost"))?, !Env::load().https_disabled())
        }
    } else {
        (Tld::new(Env::load().tld().unwrap_or("localhost"))?, !Env::load().https_disabled())
    };

    let worktree_prefix = if no_worktree {
        None
    } else {
        Worktree::detect(&cwd)
            .await
            .ok()
            .flatten()
            .and_then(|w| w.hostname_prefix())
    };
    let base = sanitize_label(&name);
    let full_host = compose_hostname(base, worktree_prefix, &tld);
    let host = Host::new(&full_host)?;
    let registry = Registry::open(paths).await?;
    if registry.get(host.as_str()).is_some() {
        println!("{}", child_url(host.as_str(), https));
    } else {
        // Not registered; print the canonical URL anyway.
        println!("{}", child_url(host.as_str(), https));
    }
    Ok(())
}

async fn cmd_alias(
    name: String,
    port: Option<u16>,
    force: bool,
    remove: bool,
    paths: Paths,
) -> Result<()> {
    let registry = Registry::open(paths).await?;
    if remove {
        let removed = registry.remove(&alias_name(&name)).await?;
        if removed.is_none() {
            return Err(Error::RouteNotFound(name));
        }
        println!("removed alias {name}");
        return Ok(());
    }
    let port = port.ok_or_else(|| Error::Config("missing port".into()))?;
    let host = alias_name(&name);
    let route = Route::alias(host, port);
    registry.insert(route, force).await?;
    println!("registered alias {name} -> 127.0.0.1:{port}");
    Ok(())
}

fn alias_name(name: &str) -> String {
    let tld = Env::load().tld().unwrap_or("localhost").to_string();
    if name.contains('.') {
        name.to_string()
    } else {
        format!("{name}.{tld}")
    }
}

async fn cmd_list(paths: Paths) -> Result<()> {
    let registry = Registry::open(paths).await?;
    let routes = registry.list();
    if routes.is_empty() {
        println!("(no routes registered)");
        return Ok(());
    }
    println!("{:<32} {:>6}  {:>8}  COMMAND", "HOSTNAME", "PORT", "PID");
    for r in routes {
        let cmd = r
            .command
            .clone()
            .unwrap_or_else(|| if r.alias { "<alias>".into() } else { "".into() });
        println!("{:<32} {:>6}  {:>8}  {}", r.hostname, r.port, r.pid, cmd);
    }
    Ok(())
}

async fn cmd_trust(paths: Paths) -> Result<()> {
    let ca = crate::tls::Ca::open(&paths).await?;
    crate::trust::install_ca(&ca).await?;
    println!("trusted local CA: {}", ca.fingerprint);
    Ok(())
}

async fn cmd_untrust(paths: Paths) -> Result<()> {
    let ca = crate::tls::Ca::open(&paths).await?;
    crate::trust::uninstall_ca(&ca).await?;
    println!("untrusted local CA");
    Ok(())
}

async fn cmd_clean(paths: Paths) -> Result<()> {
    // Stop the proxy if running.
    let _ = cmd_proxy_stop(paths.clone()).await;
    // Remove CA from trust store.
    if let Ok(ca) = crate::tls::Ca::open(&paths).await {
        let _ = crate::trust::uninstall_ca(&ca).await;
    }
    // Remove hosts block.
    let _ = hosts::clean().await;
    // Remove state directory (also removes the `logs/` subtree).
    if paths.is_inside(paths.root()) {
        let _ = tokio::fs::remove_dir_all(paths.root()).await;
    }
    // Remove the service.
    let _ = ServiceManager::detect().uninstall().await;
    println!("cleaned");
    Ok(())
}

async fn cmd_prune(force: bool, paths: Paths) -> Result<()> {
    let registry = Registry::open(paths.clone()).await?;
    let mut removed = vec![];
    let mut stale_logs = vec![];
    let routes = registry.list();
    let live_hosts: std::collections::HashSet<String> =
        routes.iter().map(|r| r.hostname.clone()).collect();
    for r in routes {
        if r.alias || r.pid == 0 {
            continue;
        }
        if !crate::process::pid_is_alive(r.pid) {
            removed.push(r.hostname.clone());
            if force {
                let _ = crate::process::signal_pid(r.pid, crate::process::Signal::Kill);
            } else {
                let _ = crate::process::kill_process(r.pid);
            }
            let _ = registry.remove(&r.hostname).await;
        }
    }
    // Sweep logs whose hostname no longer corresponds to a live app.
    let logs_dir = paths.logs_dir();
    if let Ok(mut rd) = tokio::fs::read_dir(&logs_dir).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            // `<hostname>.log` -- map back to a hostname.
            if let Some(stem) = name.strip_suffix(".log") {
                let stem = stem.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
                if !live_hosts.contains(stem) {
                    stale_logs.push(name.to_string());
                }
            }
        }
    }
    for log in &stale_logs {
        let p = logs_dir.join(log);
        let _ = tokio::fs::remove_file(&p).await;
    }
    println!("pruned {} orphan(s):", removed.len());
    for h in &removed {
        println!("  {h}");
    }
    if !stale_logs.is_empty() {
        println!("removed {} stale log file(s):", stale_logs.len());
        for l in &stale_logs {
            println!("  {l}");
        }
    }
    Ok(())
}

async fn cmd_hosts_sync(paths: Paths) -> Result<()> {
    let registry = Registry::open(paths).await?;
    let entries: Vec<HostsLine> = registry
        .list()
        .into_iter()
        .map(|r| HostsLine {
            ip: "127.0.0.1".into(),
            hostnames: vec![r.hostname],
        })
        .collect();
    hosts::sync(entries).await?;
    println!("synced /etc/hosts");
    Ok(())
}

async fn cmd_hosts_clean() -> Result<()> {
    hosts::clean().await?;
    println!("cleaned /etc/hosts");
    Ok(())
}

async fn cmd_proxy_start(opts: ProxyStartOpts, paths: Paths) -> Result<()> {
    let cfg = ProxyConfig {
        bind: format!("0.0.0.0:{}", opts.port)
            .parse()
            .map_err(|e: std::net::AddrParseError| Error::config(e.to_string()))?,
        https: !opts.no_tls,
        tld: Tld::new(opts.tld.clone())?,
        wildcard: opts.wildcard,
        cert: opts.cert.clone(),
        key: opts.key.clone(),
    };
    let router = Arc::new(crate::routing::match_::Router::new());
    let server = ProxyServer::new(cfg.clone(), paths.clone(), router).await?;
    // Persist state.
    let store = crate::state::proxy_state::Store::open(paths.clone()).await?;
    store.store(server.state()).await?;
    // Write pid file.
    let pid = std::process::id();
    tokio::fs::write(paths.proxy_pid(), pid.to_string()).await?;
    tokio::fs::write(paths.proxy_port(), opts.port.to_string()).await?;
    println!("portless proxy started on port {}", opts.port);
    if opts.foreground {
        server.run().await
    } else {
        // Daemonize: spawn and exit.
        let (paths, cfg) = (paths, cfg);
        std::process::Command::new(std::env::current_exe()?)
            .args(["proxy", "start"])
            .arg("--port")
            .arg(opts.port.to_string())
            .arg("--tld")
            .arg(opts.tld.clone())
            .arg("--foreground")
            .spawn()?;
        let _ = (paths, cfg);
        Ok(())
    }
}

async fn cmd_proxy_stop(paths: Paths) -> Result<()> {
    if let Ok(s) = tokio::fs::read_to_string(paths.proxy_pid()).await
        && let Ok(pid) = s.trim().parse::<u32>()
    {
        let _ = crate::process::signal_pid(pid, crate::process::Signal::Term);
    }
    let _ = tokio::fs::remove_file(paths.proxy_pid()).await;
    let _ = tokio::fs::remove_file(paths.proxy_port()).await;
    println!("proxy stopped");
    Ok(())
}

async fn cmd_proxy_status(paths: Paths) -> Result<()> {
    let pid = tokio::fs::read_to_string(paths.proxy_pid()).await.ok();
    let port = tokio::fs::read_to_string(paths.proxy_port()).await.ok();
    match (pid, port) {
        (Some(p), Some(port)) => {
            let pid = p.trim().parse::<u32>().unwrap_or(0);
            let alive = crate::process::pid_is_alive(pid);
            println!("proxy: pid={} port={} alive={}", pid, port.trim(), alive);
        }
        _ => println!("proxy: not running"),
    }
    Ok(())
}

async fn cmd_service_install(opts: ProxyStartOpts, _paths: Paths) -> Result<()> {
    let state = ProxyState::new(!opts.no_tls, opts.port, Tld::new(opts.tld.clone())?);
    let exe = std::env::current_exe()?;
    ServiceManager::detect().install(&state, &exe).await?;
    println!("service installed");
    Ok(())
}

async fn cmd_service_uninstall() -> Result<()> {
    ServiceManager::detect().uninstall().await?;
    println!("service uninstalled");
    Ok(())
}

async fn cmd_service_status() -> Result<()> {
    let out = ServiceManager::detect().status().await?;
    println!("{out}");
    Ok(())
}



/// Compose a hostname from an app name, optional worktree prefix, and TLD.
pub fn compose_hostname(app: String, worktree_prefix: Option<String>, tld: &Tld) -> String {
    let mut parts: Vec<String> = vec![];
    if let Some(prefix) = worktree_prefix
        && !prefix.is_empty()
    {
        parts.push(prefix);
    }
    parts.push(app);
    format!("{}.{}", parts.join("."), tld)
}

/// Tiny helper to run a shell command and surface errors.
pub fn run_shell(program: &str, args: &[&str]) -> Result<()> {
    let status = Command::new(program).args(args).status()?;
    if !status.success() {
        return Err(Error::Process(format!("{program} {}", args.join(" "))));
    }
    Ok(())
}

#[allow(dead_code)]
fn _ensure_exe_path() -> PathBuf {
    std::env::current_exe().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_strings(slice: &[&str]) -> Vec<String> {
        slice.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_direct_vite_injection() {
        let cmd = to_strings(&["vite", "dev"]);
        let result = build_command_for(&cmd, 4567, false);
        assert_eq!(
            result,
            to_strings(&["vite", "dev", "--port", "4567", "--strictPort", "--host", "127.0.0.1"])
        );
    }

    #[test]
    fn test_npx_vite_injection() {
        let cmd = to_strings(&["npx", "vite", "dev"]);
        let result = build_command_for(&cmd, 4567, false);
        assert_eq!(
            result,
            to_strings(&["npx", "vite", "dev", "--port", "4567", "--strictPort", "--host", "127.0.0.1"])
        );
    }

    #[test]
    fn test_bunx_vite_injection() {
        let cmd = to_strings(&["bunx", "--bun", "vite", "dev"]);
        let result = build_command_for(&cmd, 4567, false);
        assert_eq!(
            result,
            to_strings(&["bunx", "--bun", "vite", "dev", "--port", "4567", "--strictPort", "--host", "127.0.0.1"])
        );
    }

    #[test]
    fn test_pnpm_exec_vite_injection() {
        let cmd = to_strings(&["pnpm", "exec", "vite", "dev"]);
        let result = build_command_for(&cmd, 4567, false);
        assert_eq!(
            result,
            to_strings(&["pnpm", "exec", "vite", "dev", "--port", "4567", "--strictPort", "--host", "127.0.0.1"])
        );
    }

    #[test]
    fn test_yarn_vite_implicit_injection() {
        let cmd = to_strings(&["yarn", "vite", "dev"]);
        let result = build_command_for(&cmd, 4567, false);
        assert_eq!(
            result,
            to_strings(&["yarn", "vite", "dev", "--port", "4567", "--strictPort", "--host", "127.0.0.1"])
        );
    }

    #[test]
    fn test_yarn_dlx_vite_injection() {
        let cmd = to_strings(&["yarn", "dlx", "--yes", "vite", "dev"]);
        let result = build_command_for(&cmd, 4567, false);
        assert_eq!(
            result,
            to_strings(&["yarn", "dlx", "--yes", "vite", "dev", "--port", "4567", "--strictPort", "--host", "127.0.0.1"])
        );
    }

    #[test]
    fn test_expo_lan_mode() {
        let cmd = to_strings(&["expo", "start"]);
        let result = build_command_for(&cmd, 4567, true);
        assert_eq!(
            result,
            to_strings(&["expo", "start", "--port", "4567"])
        );
    }

    #[test]
    fn test_expo_normal_mode() {
        let cmd = to_strings(&["expo", "start"]);
        let result = build_command_for(&cmd, 4567, false);
        assert_eq!(
            result,
            to_strings(&["expo", "start", "--port", "4567", "--host", "localhost"])
        );
    }

    #[test]
    fn test_next_dev_no_injection() {
        let cmd = to_strings(&["next", "dev"]);
        let result = build_command_for(&cmd, 4567, false);
        assert_eq!(result, cmd);
    }

    #[test]
    fn test_custom_command_no_injection() {
        let cmd = to_strings(&["python", "-m", "http.server"]);
        let result = build_command_for(&cmd, 4567, false);
        assert_eq!(result, cmd);
    }
}
