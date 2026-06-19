use std::{
    io::{self, Write as _},
    path::PathBuf,
    time::Duration,
};

use clap::{Args, Parser, Subcommand, ValueEnum};
use detonger_printer::{
    protocol::PaperType, DeviceId, DiscoveredDevice, Error as PrinterError, PrintOptions,
    PrinterCaps,
};
use serde::{Deserialize, Serialize};

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

    /// Send only a limited prefix of encoded PNG vendor messages for diagnosis.
    #[command(name = "probe-png")]
    ProbePng(ProbePrintPngArgs),

    /// Print a pre-encoded Detonger job payload.
    Job(PrintJobArgs),

    /// Print a calibration pattern for width/offset.
    #[command(name = "width-test")]
    WidthTest(PrintWidthTestArgs),
}

#[derive(Debug, Subcommand)]
enum PreviewCommand {
    /// Render the width-test pattern into a PNG file.
    #[command(name = "width-test")]
    WidthTest(PreviewWidthTestArgs),

    /// Encode a PNG file into Detonger protocol packets json.
    Packets(PreviewPacketsArgs),
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
struct PreviewPacketsArgs {
    /// Input PNG file path.
    #[arg(long)]
    png: PathBuf,

    /// Output packets json file path.
    #[arg(long)]
    out: PathBuf,

    /// Print width in dots (defaults to the printhead width).
    #[arg(long)]
    width: Option<u16>,

    /// Horizontal offset in dots (negative shifts left).
    #[arg(long = "x-offset", default_value_t = 0, allow_hyphen_values = true)]
    x_offset: i16,

    /// Threshold in [0,255]. Lower values make the output lighter.
    #[arg(long, default_value_t = 150)]
    threshold: u8,

    /// Paper feed mode.
    #[arg(long = "paper-type", value_enum, default_value_t = PaperTypeArg::Gap)]
    paper_type: PaperTypeArg,

    /// Split long PNG jobs into multiple smaller row chunks.
    #[arg(long)]
    rows_per_chunk: Option<usize>,
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

    /// Paper feed mode.
    #[arg(long = "paper-type", value_enum, default_value_t = PaperTypeArg::Gap)]
    paper_type: PaperTypeArg,

    /// Split long PNG jobs into multiple smaller row chunks.
    #[arg(long)]
    rows_per_chunk: Option<usize>,
}

#[derive(Debug, Args)]
struct ProbePrintPngArgs {
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

    /// Paper feed mode.
    #[arg(long = "paper-type", value_enum, default_value_t = PaperTypeArg::Gap)]
    paper_type: PaperTypeArg,

    /// Split long PNG jobs into multiple smaller row chunks.
    #[arg(long)]
    rows_per_chunk: Option<usize>,

    /// Only send the first N vendor messages.
    #[arg(long = "max-messages")]
    max_messages: usize,
}

#[derive(Debug, Args)]
struct PrintJobArgs {
    /// Device identifier (macOS: CoreBluetooth UUID string).
    #[arg(long)]
    device: String,

    /// Path to a binary payload or packets-json file.
    #[arg(long)]
    input: PathBuf,

    /// Input file format.
    #[arg(long = "input-format", value_enum, default_value_t = PrintJobInputFormatArg::Blob)]
    input_format: PrintJobInputFormatArg,
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

    /// Paper feed mode.
    #[arg(long = "paper-type", value_enum, default_value_t = PaperTypeArg::Gap)]
    paper_type: PaperTypeArg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum PaperTypeArg {
    Continuous,
    Gap,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum PrintJobInputFormatArg {
    Blob,
    PacketsJson,
}

impl From<PaperTypeArg> for PaperType {
    fn from(value: PaperTypeArg) -> Self {
        match value {
            PaperTypeArg::Continuous => PaperType::Continuous,
            PaperTypeArg::Gap => PaperType::Gap,
        }
    }
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
    #[serde(skip_serializing_if = "Option::is_none")]
    meta: Option<serde_json::Value>,
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
        PrintCommand::ProbePng(args) => cmd_probe_print_png(format, args).await,
        PrintCommand::Job(args) => cmd_print_job(format, args).await,
        PrintCommand::WidthTest(args) => cmd_print_width_test(format, args).await,
    }
}

async fn cmd_preview(format: OutputFormat, cmd: PreviewCommand) -> ExitCode {
    match cmd {
        PreviewCommand::WidthTest(args) => cmd_preview_width_test(format, args).await,
        PreviewCommand::Packets(args) => cmd_preview_packets(format, args).await,
    }
}

async fn cmd_preview_width_test(format: OutputFormat, args: PreviewWidthTestArgs) -> ExitCode {
    let default_caps = PrinterCaps::default();
    let caps = PrinterCaps {
        dpi: default_caps.dpi,
        print_width_dots: args.width.unwrap_or(default_caps.print_width_dots),
    };

    let opts = PrintOptions {
        x_offset_dots: args.x_offset,
        ..PrintOptions::default()
    };

    let png =
        match detonger_printer::protocol::encode::render_width_test_png(&caps, &opts, args.scale) {
            Ok(v) => v,
            Err(e) => return emit_print_error(format, AppError::from_printer_error(e.into())),
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
                meta: None,
            });
        }
    }

    ExitCode::Success
}

async fn cmd_preview_packets(format: OutputFormat, args: PreviewPacketsArgs) -> ExitCode {
    let png = match std::fs::read(&args.png) {
        Ok(v) => v,
        Err(e) => {
            let err = AppError::usage(format!("failed to read png at {}: {e}", args.png.display()));
            return emit_print_error(format, err);
        }
    };

    let default_caps = PrinterCaps::default();
    let caps = PrinterCaps {
        dpi: default_caps.dpi,
        print_width_dots: args.width.unwrap_or(default_caps.print_width_dots),
    };

    let opts = PrintOptions {
        threshold: args.threshold,
        x_offset_dots: args.x_offset,
        paper_type: args.paper_type.into(),
        ..PrintOptions::default()
    };

    let packets = match args.rows_per_chunk {
        Some(rows_per_chunk) => {
            let jobs = match detonger_printer::protocol::encode_png_job_messages_in_chunks(
                &png,
                &caps,
                &opts,
                rows_per_chunk,
                detonger_printer::protocol::FinalizeMode::default(),
            ) {
                Ok(v) => v,
                Err(e) => return emit_print_error(format, AppError::from_printer_error(e.into())),
            };

            jobs.into_iter().flatten().collect::<Vec<_>>()
        }
        None => {
            match detonger_printer::protocol::encode_png_job_messages_with_finalize(
                &png,
                &caps,
                &opts,
                detonger_printer::protocol::FinalizeMode::default(),
            ) {
                Ok(v) => v,
                Err(e) => return emit_print_error(format, AppError::from_printer_error(e.into())),
            }
        }
    };

    let encoded = {
        use base64::Engine as _;

        PacketsJson {
            packets: packets
                .into_iter()
                .map(|packet| base64::engine::general_purpose::STANDARD.encode(packet))
                .collect(),
        }
    };

    let json = match serde_json::to_string_pretty(&encoded) {
        Ok(value) => value,
        Err(e) => {
            let err = AppError::usage(format!("failed to serialize packets json: {e}"));
            return emit_print_error(format, err);
        }
    };

    if let Err(e) = std::fs::write(&args.out, format!("{json}\n")) {
        let err = AppError::usage(format!(
            "failed to write packets json at {}: {e}",
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
                meta: None,
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
        paper_type: args.paper_type.into(),
        ..PrintOptions::default()
    };

    let result = if let Some(rows_per_chunk) = args.rows_per_chunk {
        conn.print_png_in_chunks(&png, &opts, rows_per_chunk).await
    } else {
        conn.print_png(&png, &opts).await
    };

    if let Err(e) = result {
        return emit_print_error(format, AppError::from_printer_error(e));
    }

    emit_print_ok(format);
    ExitCode::Success
}

async fn cmd_probe_print_png(format: OutputFormat, args: ProbePrintPngArgs) -> ExitCode {
    let png = match std::fs::read(&args.png) {
        Ok(v) => v,
        Err(e) => {
            let err = AppError::usage(format!("failed to read png at {}: {e}", args.png.display()));
            return emit_print_error(format, err);
        }
    };

    let messages = match encode_png_messages(
        &png,
        args.threshold,
        args.x_offset,
        args.paper_type,
        args.rows_per_chunk,
    ) {
        Ok(messages) => messages,
        Err(err) => return emit_print_error(format, err),
    };

    let total_messages = messages.len();
    let limited_messages = args.max_messages.min(total_messages);
    let meta = serde_json::json!({
        "stage": if limited_messages == 0 { "connected" } else { "write-complete" },
        "messageCount": limited_messages,
        "totalMessageCount": total_messages,
    });

    let device = DeviceId(args.device);
    let mut conn = match detonger_printer::connect(&device).await {
        Ok(v) => v,
        Err(e) => {
            return emit_print_error_with_meta(
                format,
                AppError::from_printer_error(e),
                serde_json::json!({
                    "stage": "connect",
                    "messageCount": 0,
                    "totalMessageCount": total_messages,
                }),
            )
        }
    };

    if limited_messages == 0 {
        emit_print_ok_with_meta(format, meta);
        return ExitCode::Success;
    }

    if let Err(e) = conn.print_vendor_messages(&messages[..limited_messages]).await {
        return emit_print_error_with_meta(
            format,
            AppError::from_printer_error(e),
            serde_json::json!({
                "stage": "write",
                "messageCount": limited_messages,
                "totalMessageCount": total_messages,
            }),
        );
    }

    emit_print_ok_with_meta(format, meta);
    ExitCode::Success
}

async fn cmd_print_job(format: OutputFormat, args: PrintJobArgs) -> ExitCode {
    let device = DeviceId(args.device);
    let mut conn = match detonger_printer::connect(&device).await {
        Ok(v) => v,
        Err(e) => return emit_print_error(format, AppError::from_printer_error(e)),
    };

    let result = match args.input_format {
        PrintJobInputFormatArg::Blob => match std::fs::read(&args.input) {
            Ok(payload) => conn.print_job_payload(&payload).await,
            Err(e) => {
                let err = AppError::usage(format!(
                    "failed to read job payload at {}: {e}",
                    args.input.display()
                ));
                return emit_print_error(format, err);
            }
        },
        PrintJobInputFormatArg::PacketsJson => {
            let payload = match std::fs::read_to_string(&args.input) {
                Ok(value) => value,
                Err(e) => {
                    let err = AppError::usage(format!(
                        "failed to read packets json at {}: {e}",
                        args.input.display()
                    ));
                    return emit_print_error(format, err);
                }
            };

            let parsed: PacketsJson = match serde_json::from_str(&payload) {
                Ok(value) => value,
                Err(e) => {
                    let err = AppError::usage(format!(
                        "failed to parse packets json at {}: {e}",
                        args.input.display()
                    ));
                    return emit_print_error(format, err);
                }
            };

            let packets = match parsed.decode_packets() {
                Ok(value) => value,
                Err(e) => return emit_print_error(format, AppError::from_printer_error(e)),
            };

            let mut messages = Vec::new();
            for packet in packets {
                let mut split = match detonger_printer::protocol::split_vendor_messages(&packet) {
                    Ok(value) => value,
                    Err(e) => return emit_print_error(format, AppError::from_printer_error(e.into())),
                };
                messages.append(&mut split);
            }

            conn.print_vendor_messages(&messages).await
        }
    };

    if let Err(e) = result {
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
        x_offset_dots: args.x_offset,
        paper_type: args.paper_type.into(),
        ..PrintOptions::default()
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
    emit_print_ok_with_meta(format, serde_json::Value::Null);
}

fn emit_print_ok_with_meta(format: OutputFormat, meta: serde_json::Value) {
    match format {
        OutputFormat::Human => println!("ok"),
        OutputFormat::Json => {
            let _ = write_json_stdout(&JsonCommandResult {
                status: "ok",
                out: None,
                error: None,
                meta: if meta.is_null() { None } else { Some(meta) },
            });
        }
    }
}

fn emit_print_error(format: OutputFormat, err: AppError) -> ExitCode {
    emit_print_error_with_meta(format, err, serde_json::Value::Null)
}

fn emit_print_error_with_meta(
    format: OutputFormat,
    err: AppError,
    meta: serde_json::Value,
) -> ExitCode {
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
                meta: if meta.is_null() { None } else { Some(meta) },
            });
        }
    }

    err.code
}

fn encode_png_messages(
    png: &[u8],
    threshold: u8,
    x_offset: i16,
    paper_type: PaperTypeArg,
    rows_per_chunk: Option<usize>,
) -> Result<Vec<Vec<u8>>, AppError> {
    let opts = PrintOptions {
        threshold,
        x_offset_dots: x_offset,
        paper_type: paper_type.into(),
        ..PrintOptions::default()
    };

    match rows_per_chunk {
        Some(rows_per_chunk) => {
            let jobs = detonger_printer::protocol::encode_png_job_messages_in_chunks(
                png,
                &PrinterCaps::default(),
                &opts,
                rows_per_chunk,
                detonger_printer::protocol::FinalizeMode::default(),
            )
            .map_err(|e| AppError::from_printer_error(e.into()))?;

            Ok(jobs.into_iter().flatten().collect())
        }
        None => detonger_printer::protocol::encode_png_job_messages_with_finalize(
            png,
            &PrinterCaps::default(),
            &opts,
            detonger_printer::protocol::FinalizeMode::default(),
        )
        .map_err(|e| AppError::from_printer_error(e.into())),
    }
}

fn write_json_stdout<T: Serialize>(value: &T) -> Result<(), AppError> {
    let mut out = io::stdout().lock();
    serde_json::to_writer(&mut out, value)
        .map_err(|e| AppError::usage(format!("failed to write json: {e}")))?;
    out.write_all(b"\n")
        .map_err(|e| AppError::usage(format!("failed to write json: {e}")))?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
struct PacketsJson {
    packets: Vec<String>,
}

impl PacketsJson {
    fn decode_packets(self) -> Result<Vec<Vec<u8>>, PrinterError> {
        if self.packets.is_empty() {
            return Err(PrinterError::InvalidArgument(
                "packets json must contain at least one packet".into(),
            ));
        }

        self.packets
            .into_iter()
            .enumerate()
            .map(|(index, packet)| {
                use base64::Engine as _;
                base64::engine::general_purpose::STANDARD
                    .decode(packet)
                    .map_err(|err| {
                        PrinterError::InvalidArgument(format!(
                            "packets[{index}] base64 decode failed: {err}"
                        ))
                    })
            })
            .collect()
    }
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
            "--paper-type",
            "continuous",
            "--rows-per-chunk",
            "4",
        ])
        .unwrap();

        match cli.command {
            Command::Print(PrintCommand::Png(args)) => {
                assert_eq!(args.device, "dev1");
                assert_eq!(args.png, PathBuf::from("a.png"));
                assert_eq!(args.x_offset, -5);
                assert_eq!(args.threshold, 200);
                assert_eq!(args.paper_type, PaperTypeArg::Continuous);
                assert_eq!(args.rows_per_chunk, Some(4));
            }
            other => panic!("unexpected parse: {other:?}"),
        }
    }

    #[test]
    fn clap_parses_print_job_flags() {
        let cli = Cli::try_parse_from([
            "detonger",
            "print",
            "job",
            "--device",
            "dev1",
            "--input",
            "job.json",
            "--input-format",
            "packets-json",
        ])
        .unwrap();

        match cli.command {
            Command::Print(PrintCommand::Job(args)) => {
                assert_eq!(args.device, "dev1");
                assert_eq!(args.input, PathBuf::from("job.json"));
                assert_eq!(args.input_format, PrintJobInputFormatArg::PacketsJson);
            }
            other => panic!("unexpected parse: {other:?}"),
        }
    }

    #[test]
    fn clap_parses_probe_print_png_flags() {
        let cli = Cli::try_parse_from([
            "detonger",
            "print",
            "probe-png",
            "--device",
            "dev1",
            "--png",
            "probe.png",
            "--x-offset",
            "-3",
            "--threshold",
            "170",
            "--paper-type",
            "continuous",
            "--rows-per-chunk",
            "8",
            "--max-messages",
            "12",
        ])
        .unwrap();

        match cli.command {
            Command::Print(PrintCommand::ProbePng(args)) => {
                assert_eq!(args.device, "dev1");
                assert_eq!(args.png, PathBuf::from("probe.png"));
                assert_eq!(args.x_offset, -3);
                assert_eq!(args.threshold, 170);
                assert_eq!(args.paper_type, PaperTypeArg::Continuous);
                assert_eq!(args.rows_per_chunk, Some(8));
                assert_eq!(args.max_messages, 12);
            }
            other => panic!("unexpected parse: {other:?}"),
        }
    }

    #[test]
    fn clap_parses_preview_packets_flags() {
        let cli = Cli::try_parse_from([
            "detonger",
            "preview",
            "packets",
            "--png",
            "a.png",
            "--out",
            "job.json",
            "--width",
            "384",
            "--x-offset",
            "-7",
            "--threshold",
            "180",
            "--paper-type",
            "continuous",
            "--rows-per-chunk",
            "24",
        ])
        .unwrap();

        match cli.command {
            Command::Preview(PreviewCommand::Packets(args)) => {
                assert_eq!(args.png, PathBuf::from("a.png"));
                assert_eq!(args.out, PathBuf::from("job.json"));
                assert_eq!(args.width, Some(384));
                assert_eq!(args.x_offset, -7);
                assert_eq!(args.threshold, 180);
                assert_eq!(args.paper_type, PaperTypeArg::Continuous);
                assert_eq!(args.rows_per_chunk, Some(24));
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
