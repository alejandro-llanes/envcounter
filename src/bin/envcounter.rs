use clap::{Parser, Subcommand, ValueEnum};
use envcounter::protocol::{Request, Response};

#[derive(Parser)]
#[command(name = "envcounter", about = "Manage counter and UUID pools")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Manage counters
    Counter {
        #[command(subcommand)]
        action: CounterAction,
    },
    /// Manage UUID pools
    Uuid {
        #[command(subcommand)]
        action: UuidAction,
    },
    /// List pools
    List {
        /// Output format
        #[arg(long, short, default_value = "table")]
        format: OutputFormat,
        /// Filter by pool type
        #[arg(long, short = 't')]
        r#type: Option<PoolTypeFilter>,
    },
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Table,
    Json,
}

#[derive(Clone, ValueEnum)]
enum PoolTypeFilter {
    Counter,
    Uuid,
}

#[derive(Subcommand)]
enum CounterAction {
    /// Create a new counter
    New {
        name: String,
        /// Increment step (default: 1)
        #[arg(long, short)]
        step: Option<i64>,
        /// Initial value (default: 0)
        #[arg(long, short)]
        initial: Option<i64>,
    },
    /// Read current value and increment
    Read { name: String },
    /// Read current value without incrementing
    Seek { name: String },
    /// Remove a counter
    Rm { name: String },
    /// Rename a counter
    Rename { name: String, new_name: String },
    /// Reset a counter to its initial value
    Reset { name: String },
}

#[derive(Subcommand)]
enum UuidAction {
    /// Create a new UUID pool
    New { name: String },
    /// Generate a new UUID
    Read { name: String },
    /// Remove a UUID pool
    Rm { name: String },
}

fn to_request(cmd: &Command) -> Request {
    match cmd {
        Command::Counter { action } => match action {
            CounterAction::New {
                name,
                step,
                initial,
            } => Request::CounterNew {
                name: name.clone(),
                step: *step,
                initial: *initial,
            },
            CounterAction::Read { name } => Request::CounterRead { name: name.clone() },
            CounterAction::Seek { name } => Request::CounterSeek { name: name.clone() },
            CounterAction::Rm { name } => Request::CounterRm { name: name.clone() },
            CounterAction::Rename { name, new_name } => Request::CounterRename {
                name: name.clone(),
                new_name: new_name.clone(),
            },
            CounterAction::Reset { name } => Request::CounterReset { name: name.clone() },
        },
        Command::Uuid { action } => match action {
            UuidAction::New { name } => Request::UuidNew { name: name.clone() },
            UuidAction::Read { name } => Request::UuidRead { name: name.clone() },
            UuidAction::Rm { name } => Request::UuidRm { name: name.clone() },
        },
        Command::List { r#type, .. } => Request::List {
            filter_type: r#type.as_ref().map(|t| match t {
                PoolTypeFilter::Counter => "counter".to_string(),
                PoolTypeFilter::Uuid => "uuid".to_string(),
            }),
        },
    }
}

fn print_response(response: Response, cmd: &Command) {
    match response {
        Response::Ok { value: None } => {}
        Response::Ok { value: Some(v) } => {
            if let Command::List { format, .. } = cmd {
                print_list(&v, format);
            } else if let Some(s) = v.as_str() {
                println!("{}", s);
            } else if let Some(n) = v.as_i64() {
                println!("{}", n);
            } else {
                println!("{}", v);
            }
        }
        Response::Error { message } => {
            eprintln!("error: {}", message);
            std::process::exit(1);
        }
    }
}

fn print_list(value: &serde_json::Value, format: &OutputFormat) {
    let arr = match value.as_array() {
        Some(a) => a,
        None => {
            println!("{}", value);
            return;
        }
    };

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(arr).unwrap());
        }
        OutputFormat::Table => {
            if arr.is_empty() {
                println!("(no pools)");
                return;
            }

            // Collect rows: (name, type, details)
            let rows: Vec<(String, String, String)> = arr
                .iter()
                .map(|item| {
                    let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                    let kind = item.get("type").and_then(|v| v.as_str()).unwrap_or("?");
                    let details = match kind {
                        "counter" => {
                            let val = item.get("value").and_then(|v| v.as_i64()).unwrap_or(0);
                            let step = item.get("step").and_then(|v| v.as_i64()).unwrap_or(0);
                            let initial = item.get("initial").and_then(|v| v.as_i64()).unwrap_or(0);
                            format!("value={} step={} initial={}", val, step, initial)
                        }
                        _ => String::from("-"),
                    };
                    (name.to_string(), kind.to_string(), details)
                })
                .collect();

            let name_w = rows.iter().map(|r| r.0.len()).max().unwrap_or(4).max(4);
            let type_w = rows.iter().map(|r| r.1.len()).max().unwrap_or(4).max(4);

            println!(
                "{:<name_w$}  {:<type_w$}  {}",
                "NAME", "TYPE", "DETAILS"
            );
            for (name, kind, details) in &rows {
                println!("{:<name_w$}  {:<type_w$}  {}", name, kind, details);
            }
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let cli = Cli::parse();
    let request = to_request(&cli.command);

    match envcounter::client::send(&request).await {
        Ok(response) => print_response(response, &cli.command),
        Err(e) => {
            eprintln!("error: {}", e);
            std::process::exit(1);
        }
    }
}
