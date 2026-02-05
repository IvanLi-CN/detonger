use std::{
    io::{self, Write as _},
    path::PathBuf,
    time::Duration,
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use detonger_printer::{
    DeviceId, DiscoveredDevice, Error as PrinterError, PrintOptions, PrinterCaps,
};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum OutputFormat {
    Human,
    Json,
}

#[derive(Debug, Parser)]
#[command(name = "detonger", about = "CLI for DeTong / Detonger label printers")]
struct Cli {
    /// Output format (applies to all commands).
    #[arg(long, value_enum, default_value_t = OutputFormat::Human, global = true)]
    format: OutputFormat,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Scan for nearby printers.
    Scan(ScanArgs),

    /// Print operations.
    #[command(subcommand)]
    Print(PrintCommand),

    /// Generate previews without talking to the printer.
    #[command(subcommand)]
    Preview(PreviewCommand),
}

#[derive(Debug, Args)]
struct ScanArgs {
    /// Scan timeout in seconds.
    #[arg(long = "timeout-s", default_value_t = 5)]
    timeout_s: u64,
}

#[derive(Debug, Subcommand)]
enum PrintCommand {
    /// Print a PNG file.
    Png(PrintPngArgs),

    /// Print a calibration pattern for width/offset.
    #[command(name = "width-test")]
    WidthTest(PrintWidthTestArgs),
}

#[derive(Debug, Subcommand)]
enum PreviewCommand {
    /// Render the width-test pattern into a PNG file.
    #[command(name = "width-test")]
    WidthTest(PreviewWidthTestArgs),
}

#[derive(Debug, Args)]
struct PreviewWidthTestArgs {
    /// Output PNG file path.
    #[arg(long)]
    out: PathBuf,

    /// Pattern width in dots (defaults to the printhead width).
    #[arg(long)]
    width: Option<u16>,

    /// Horizontal offset in dots (negative shifts left).
    #[arg(long = "x-offset", default_value_t = 0, allow_hyphen_values = true)]
    x_offset: i16,

    /// PNG preview scale factor (4 makes the pattern easier to see).
    #[arg(long, default_value_t = 4)]
    scale: u32,
}

#[derive(Debug, Args)]
struct PrintPngArgs {
    /// Device identifier (macOS: CoreBluetooth UUID string).
    #[arg(long)]
    device: String,

    /// Path to PNG file.
    #[arg(long)]
    png: PathBuf,

    /// Horizontal offset in dots (negative shifts left).
    #[arg(long = "x-offset", default_value_t = 0, allow_hyphen_values = true)]
    x_offset: i16,

    /// Threshold in [0,255]. Lower values make the output lighter.
    #[arg(long, default_value_t = 150)]
    threshold: u8,
}

#[derive(Debug, Args)]
struct PrintWidthTestArgs {
    /// Device identifier (macOS: CoreBluetooth UUID string).
    #[arg(long)]
    device: String,

    /// Pattern width in dots (defaults to the printhead width).
    #[arg(long)]
    width: Option<u16>,

    /// Pattern height in dots (currently reserved for future use).
    #[arg(long)]
    height: Option<u16>,

    /// Horizontal offset in dots (negative shifts left).
    #[arg(long = "x-offset", default_value_t = 0, allow_hyphen_values = true)]
    x_offset: i16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExitCode {
    Success = 0,
    Usage = 2,
    BleOrConnect = 10,
    PrinterNotAvailable = 20,
    Unknown = 100,
}

impl ExitCode {
    fn as_i32(self) -> i32 {
        self as i32
    }
}

#[derive(Debug)]
struct AppError {
    code: ExitCode,
    kind: &'static str,
    message: String,
}

impl AppError {
    fn from_printer_error(err: PrinterError) -> Self {
        match err {
            PrinterError::InvalidArgument(msg) => Self {
                code: ExitCode::Usage,
                kind: "invalid_argument",
                message: msg,
            },
            PrinterError::NotFound(msg) => Self {
                code: ExitCode::BleOrConnect,
                kind: "not_found",
                message: msg,
            },
            PrinterError::Timeout => Self {
                code: ExitCode::BleOrConnect,
                kind: "timeout",
                message: "timeout".to_string(),
            },
            PrinterError::PrinterNotAvailable(msg) => Self {
                code: ExitCode::PrinterNotAvailable,
                kind: "printer_not_available",
                message: msg,
            },
            PrinterError::Ble(msg) => Self {
                code: ExitCode::BleOrConnect,
                kind: "ble",
                message: msg,
            },
            other => Self {
                code: ExitCode::Unknown,
                kind: "unknown",
                message: other.to_string(),
            },
        }
    }

    fn usage(message: impl Into<String>) -> Self {
        Self {
            code: ExitCode::Usage,
            kind: "usage",
            message: message.into(),
        }
    }
}

#[derive(Debug, Serialize)]
struct JsonScanDevice<'a> {
    device: &'a str,
    name: &'a Option<String>,
    rssi: &'a Option<i16>,
}

#[derive(Debug, Serialize)]
struct JsonCommandResult {
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    out: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonError>,
}

#[derive(Debug, Serialize)]
struct JsonError {
    kind: &'static str,
    message: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let exit_code = match cli.command {
        Command::Scan(args) => cmd_scan(cli.format, args).await,
        Command::Print(cmd) => cmd_print(cli.format, cmd).await,
        Command::Preview(cmd) => cmd_preview(cli.format, cmd).await,
    };

    std::process::exit(exit_code.as_i32());
}

async fn cmd_scan(format: OutputFormat, args: ScanArgs) -> ExitCode {
    let timeout = Duration::from_secs(args.timeout_s);

    match detonger_printer::scan(timeout).await {
        Ok(devices) => {
            match format {
                OutputFormat::Human => emit_scan_human(&devices),
                OutputFormat::Json => {
                    if emit_scan_json(&devices).is_err() {
                        return ExitCode::Unknown;
                    }
                }
            }
            ExitCode::Success
        }
        Err(err) => {
            let app_err = AppError::from_printer_error(err);

            // Keep stdout stable for machine parsing.
            if format == OutputFormat::Json {
                let _ = write_json_stdout(&Vec::<JsonScanDevice<'_>>::new());
            }

            let _ = writeln!(
                io::stderr(),
                "error({}): {}",
                app_err.code.as_i32(),
                app_err.message
            );

            app_err.code
        }
    }
}

async fn cmd_print(format: OutputFormat, cmd: PrintCommand) -> ExitCode {
    match cmd {
        PrintCommand::Png(args) => cmd_print_png(format, args).await,
        PrintCommand::WidthTest(args) => cmd_print_width_test(format, args).await,
    }
}

async fn cmd_preview(format: OutputFormat, cmd: PreviewCommand) -> ExitCode {
    match cmd {
        PreviewCommand::WidthTest(args) => cmd_preview_width_test(format, args).await,
    }
}

async fn cmd_preview_width_test(format: OutputFormat, args: PreviewWidthTestArgs) -> ExitCode {
    let default_caps = PrinterCaps::default();
    let caps = PrinterCaps {
        dpi: default_caps.dpi,
        print_width_dots: args.width.unwrap_or(default_caps.print_width_dots),
    };

    let opts = PrintOptions {
        threshold: PrintOptions::default().threshold,
        x_offset_dots: args.x_offset,
    };

    let png =
        match detonger_printer::protocol::encode::render_width_test_png(&caps, &opts, args.scale) {
            Ok(v) => v,
            Err(e) => return emit_print_error(format, AppError::from_printer_error(e)),
        };

    if let Err(e) = std::fs::write(&args.out, &png) {
        let err = AppError::usage(format!(
            "failed to write png at {}: {e}",
            args.out.display()
        ));
        return emit_print_error(format, err);
    }

    match format {
        OutputFormat::Human => println!("wrote {}", args.out.display()),
        OutputFormat::Json => {
            let _ = write_json_stdout(&JsonCommandResult {
                status: "ok",
                out: Some(args.out.display().to_string()),
                error: None,
            });
        }
    }

    ExitCode::Success
}

async fn cmd_print_png(format: OutputFormat, args: PrintPngArgs) -> ExitCode {
    let png = match std::fs::read(&args.png) {
        Ok(v) => v,
        Err(e) => {
            let err = AppError::usage(format!("failed to read png at {}: {e}", args.png.display()));
            return emit_print_error(format, err);
        }
    };

    let device = DeviceId(args.device);
    let mut conn = match detonger_printer::connect(&device).await {
        Ok(v) => v,
        Err(e) => return emit_print_error(format, AppError::from_printer_error(e)),
    };

    let opts = PrintOptions {
        threshold: args.threshold,
        x_offset_dots: args.x_offset,
    };

    if let Err(e) = conn.print_png(&png, &opts).await {
        return emit_print_error(format, AppError::from_printer_error(e));
    }

    emit_print_ok(format);
    ExitCode::Success
}

async fn cmd_print_width_test(format: OutputFormat, args: PrintWidthTestArgs) -> ExitCode {
    let device = DeviceId(args.device);
    let mut conn = match detonger_printer::connect(&device).await {
        Ok(v) => v,
        Err(e) => return emit_print_error(format, AppError::from_printer_error(e)),
    };

    // Note: `height` is parsed for CLI contract completeness; it's reserved until the printer API
    // adds an explicit knob for it.
    let _height = args.height;

    let default_caps = PrinterCaps::default();
    let caps = PrinterCaps {
        dpi: default_caps.dpi,
        print_width_dots: args.width.unwrap_or(default_caps.print_width_dots),
    };

    let opts = PrintOptions {
        threshold: PrintOptions::default().threshold,
        x_offset_dots: args.x_offset,
    };

    if let Err(e) = conn.print_width_test(&caps, &opts).await {
        return emit_print_error(format, AppError::from_printer_error(e));
    }

    emit_print_ok(format);
    ExitCode::Success
}

fn emit_scan_human(devices: &[DiscoveredDevice]) {
    println!("found {} device(s)", devices.len());
    for d in devices {
        let id = &d.id.0;
        let name = d.name.as_deref().unwrap_or("-");
        let rssi = d
            .rssi
            .map(|v| v.to_string())
            .unwrap_or_else(|| "-".to_string());
        println!("{id}\t{name}\t{rssi}");
    }
}

fn emit_scan_json(devices: &[DiscoveredDevice]) -> Result<(), AppError> {
    let out: Vec<JsonScanDevice<'_>> = devices
        .iter()
        .map(|d| JsonScanDevice {
            device: d.id.0.as_str(),
            name: &d.name,
            rssi: &d.rssi,
        })
        .collect();

    write_json_stdout(&out)
}

fn emit_print_ok(format: OutputFormat) {
    match format {
        OutputFormat::Human => println!("ok"),
        OutputFormat::Json => {
            let _ = write_json_stdout(&JsonCommandResult {
                status: "ok",
                out: None,
                error: None,
            });
        }
    }
}

fn emit_print_error(format: OutputFormat, err: AppError) -> ExitCode {
    match format {
        OutputFormat::Human => {
            let _ = writeln!(
                io::stderr(),
                "error({}): {}",
                err.code.as_i32(),
                err.message
            );
        }
        OutputFormat::Json => {
            let _ = write_json_stdout(&JsonCommandResult {
                status: "error",
                out: None,
                error: Some(JsonError {
                    kind: err.kind,
                    message: err.message.clone(),
                }),
            });
        }
    }

    err.code
}

fn write_json_stdout<T: Serialize>(value: &T) -> Result<(), AppError> {
    let mut out = io::stdout().lock();
    serde_json::to_writer(&mut out, value)
        .map_err(|e| AppError::usage(format!("failed to write json: {e}")))?;
    out.write_all(b"\n")
        .map_err(|e| AppError::usage(format!("failed to write json: {e}")))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clap_parses_print_png_flags() {
        let cli = Cli::try_parse_from([
            "detonger",
            "print",
            "png",
            "--device",
            "dev1",
            "--png",
            "a.png",
            "--x-offset",
            "-5",
            "--threshold",
            "200",
        ])
        .unwrap();

        match cli.command {
            Command::Print(PrintCommand::Png(args)) => {
                assert_eq!(args.device, "dev1");
                assert_eq!(args.png, PathBuf::from("a.png"));
                assert_eq!(args.x_offset, -5);
                assert_eq!(args.threshold, 200);
            }
            other => panic!("unexpected parse: {other:?}"),
        }
    }

    #[test]
    fn printer_error_exit_code_mapping() {
        let e = AppError::from_printer_error(PrinterError::Ble("x".into()));
        assert_eq!(e.code, ExitCode::BleOrConnect);

        let e = AppError::from_printer_error(PrinterError::PrinterNotAvailable("x".into()));
        assert_eq!(e.code, ExitCode::PrinterNotAvailable);

        let e = AppError::from_printer_error(PrinterError::InvalidArgument("x".into()));
        assert_eq!(e.code, ExitCode::Usage);
    }
}
