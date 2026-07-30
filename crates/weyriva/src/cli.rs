use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::Arc;

use clap::error::ErrorKind;
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json::{Value as JsonValue, json};

use crate::broker::Broker;
use crate::diagnose::{Diagnoser, render_plain};
use crate::error::{Error, Result};
use crate::identity::SystemIdentity;
use crate::ipc;
use crate::lock::LockReconciler;
use crate::paths::Paths;
use crate::process::SystemProcess;
use crate::session::SessionController;
use crate::shell::ShellController;
use crate::startup::{StartupContext, apply, preflight};

#[derive(Debug, Parser)]
#[command(name = "weyriva", version, about = "Weyriva Shell control plane")]
struct CommandLine {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Daemon,
    Status,
    Diagnose {
        #[arg(long)]
        json: bool,
    },
    Startup(StartupArgs),
    Ipc(IpcArgs),
    Shell(ShellArgs),
    Session(SessionArgs),
    Plugin(PluginArgs),
}

#[derive(Debug, Args)]
struct StartupArgs {
    #[command(subcommand)]
    command: StartupCommand,
}

#[derive(Debug, Subcommand)]
enum StartupCommand {
    Ensure {
        #[arg(long)]
        user: Option<String>,
    },
}

#[derive(Debug, Args)]
struct IpcArgs {
    #[command(subcommand)]
    command: IpcCommand,
}

#[derive(Debug, Subcommand)]
enum IpcCommand {
    Call {
        method: String,
        #[arg(long, default_value = "{}", value_parser = parse_json)]
        params: JsonValue,
    },
}

#[derive(Debug, Args)]
struct ShellArgs {
    #[command(subcommand)]
    command: ShellCommand,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum Route {
    Launcher,
    ControlCenter,
    Calendar,
    Notifications,
    Wallpaper,
    Settings,
}

impl Route {
    const fn name(self) -> &'static str {
        match self {
            Self::Launcher => "launcher",
            Self::ControlCenter => "control-center",
            Self::Calendar => "calendar",
            Self::Notifications => "notifications",
            Self::Wallpaper => "wallpaper",
            Self::Settings => "settings",
        }
    }
}

#[derive(Debug, Subcommand)]
enum ShellCommand {
    Run,
    ReconcileLock,
    Route { name: Route },
    Lock,
}

#[derive(Debug, Args)]
struct SessionArgs {
    #[command(subcommand)]
    command: SessionCommand,
}

#[derive(Debug, Subcommand)]
enum SessionCommand {
    Start,
}

#[derive(Debug, Args)]
struct PluginArgs {
    #[command(subcommand)]
    command: PluginCommand,
}

#[derive(Debug, Subcommand)]
enum PluginCommand {
    Source(SourceArgs),
    Install {
        id: String,
    },
    Status {
        id: Option<String>,
    },
    Enable {
        id: String,
    },
    Disable {
        id: String,
    },
    Reload {
        id: String,
    },
    Uninstall {
        id: String,
    },
    #[command(hide = true)]
    Query {
        provider: String,
        text: String,
    },
    #[command(hide = true)]
    Activate {
        provider: String,
        result_id: String,
    },
    #[command(hide = true)]
    Ipc {
        entry: String,
        event: String,
        #[arg(long, default_value = "{}", value_parser = parse_json)]
        payload: JsonValue,
    },
}

#[derive(Debug, Args)]
struct SourceArgs {
    #[command(subcommand)]
    command: SourceCommand,
}

#[derive(Debug, Subcommand)]
enum SourceCommand {
    List,
    Add { name: String, path: PathBuf },
    Remove { name: String },
}

/// Runs the Weyriva command line from the process argument vector.
///
/// # Errors
///
/// Returns a runtime, IPC, startup, or output error from the selected command.
pub fn run() -> Result<i32> {
    run_from(std::env::args_os())
}

/// Parses and runs a supplied Weyriva argument vector.
///
/// # Errors
///
/// Returns a runtime, IPC, startup, or output error from the selected command.
pub fn run_from(arguments: impl IntoIterator<Item = OsString>) -> Result<i32> {
    let arguments: Vec<OsString> = arguments.into_iter().collect();
    let command = match CommandLine::try_parse_from(arguments.clone()) {
        Ok(command) => command,
        Err(error) => return clap_result(&error),
    };
    execute(
        command.command,
        arguments
            .first()
            .cloned()
            .unwrap_or_else(|| OsString::from("weyriva")),
    )
}

fn execute(command: Command, argv0: OsString) -> Result<i32> {
    match command {
        Command::Daemon => {
            let paths = Paths::from_env()?;
            ipc::serve(&paths, Broker::new(paths.clone()))?;
            Ok(0)
        }
        Command::Status => daemon_call("weyriva.ping", &json!({})),
        Command::Ipc(ipc) => match ipc.command {
            IpcCommand::Call { method, params } => daemon_call(&method, &params),
        },
        Command::Plugin(plugin) => {
            let (method, params) = plugin_request(plugin.command);
            daemon_call(method, &params)
        }
        Command::Diagnose { json } => {
            let report = Diagnoser::system().run();
            if json {
                print_json(&serde_json::to_value(&report)?);
            } else {
                print!("{}", render_plain(&report));
            }
            Ok(i32::from(!report.ok))
        }
        Command::Shell(shell) => shell_command(&shell.command),
        Command::Session(session) => match session.command {
            SessionCommand::Start => {
                SessionController::from_environment(argv0).start()?;
                Ok(0)
            }
        },
        Command::Startup(startup) => match startup.command {
            StartupCommand::Ensure { user } => {
                let context = StartupContext::system();
                let target = user.or_else(|| {
                    context
                        .environment
                        .get(std::ffi::OsStr::new("SUDO_USER"))
                        .map(|value| value.to_string_lossy().into_owned())
                });
                let target = target.unwrap_or_default();
                let plan = preflight(&context, &target)?;
                let result = apply(&context, &plan)?;
                print!("{}", result.output);
                Ok(0)
            }
        },
    }
}

fn shell_command(command: &ShellCommand) -> Result<i32> {
    let shell = ShellController::from_environment();
    match command {
        ShellCommand::Run => {
            shell.run()?;
            Ok(0)
        }
        ShellCommand::ReconcileLock => {
            let environment = std::env::vars_os().collect();
            let reconciler = LockReconciler::new(Arc::new(SystemProcess), Arc::new(SystemIdentity));
            Ok(i32::from(!reconciler.reconcile(&environment)))
        }
        ShellCommand::Route { name } => {
            print_json(&shell.call("route", &[name.name()])?);
            Ok(0)
        }
        ShellCommand::Lock => {
            print_json(&shell.call("lock", &[])?);
            Ok(0)
        }
    }
}

fn daemon_call(method: &str, params: &JsonValue) -> Result<i32> {
    let paths = Paths::from_env()?;
    let response = ipc::call(&paths.socket_file(), method, params)?;
    print_json(&response);
    Ok(i32::from(response.get("result").is_none()))
}

fn plugin_request(command: PluginCommand) -> (&'static str, JsonValue) {
    match command {
        PluginCommand::Source(source) => match source.command {
            SourceCommand::List => ("weyriva.plugin.v1.source.list", json!({})),
            SourceCommand::Add { name, path } => (
                "weyriva.plugin.v1.source.add",
                json!({"name": name, "path": path}),
            ),
            SourceCommand::Remove { name } => {
                ("weyriva.plugin.v1.source.remove", json!({"name": name}))
            }
        },
        PluginCommand::Install { id } => ("weyriva.plugin.v1.install", json!({"id": id})),
        PluginCommand::Status { id } => (
            "weyriva.plugin.v1.status",
            id.map_or_else(|| json!({}), |id| json!({"id": id})),
        ),
        PluginCommand::Enable { id } => ("weyriva.plugin.v1.enable", json!({"id": id})),
        PluginCommand::Disable { id } => ("weyriva.plugin.v1.disable", json!({"id": id})),
        PluginCommand::Reload { id } => ("weyriva.plugin.v1.reload", json!({"id": id})),
        PluginCommand::Uninstall { id } => ("weyriva.plugin.v1.uninstall", json!({"id": id})),
        PluginCommand::Query { provider, text } => (
            "weyriva.plugin.v1.query",
            json!({"provider": provider, "query": text}),
        ),
        PluginCommand::Activate {
            provider,
            result_id,
        } => (
            "weyriva.plugin.v1.activate",
            json!({"provider": provider, "result_id": result_id}),
        ),
        PluginCommand::Ipc {
            entry,
            event,
            payload,
        } => (
            "weyriva.plugin.v1.ipc",
            json!({"entry": entry, "event": event, "payload": payload}),
        ),
    }
}

fn parse_json(value: &str) -> std::result::Result<JsonValue, String> {
    serde_json::from_str(value).map_err(|error| format!("invalid JSON: {error}"))
}

fn print_json(value: &JsonValue) {
    match serde_json::to_string_pretty(value) {
        Ok(encoded) => println!("{encoded}"),
        Err(error) => eprintln!("weyriva: cannot encode JSON: {error}"),
    }
}

fn clap_result(error: &clap::Error) -> Result<i32> {
    let code = if matches!(
        error.kind(),
        ErrorKind::DisplayHelp | ErrorKind::DisplayVersion
    ) {
        0
    } else {
        2
    };
    error
        .print()
        .map_err(|io| Error::io("cannot print command help", &io))?;
    Ok(code)
}
