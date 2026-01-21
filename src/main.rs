use std::convert::Infallible;
use std::net::SocketAddr;

use clap::Parser;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use log::{debug, error, info, warn};
use p1_prometheus_exporter_rs::process_p1_line;
use prometheus::{Encoder, TextEncoder};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use tokio_serial::{DataBits, Parity, SerialPortBuilderExt, StopBits};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Port the Prometheus exporter should listen on
    #[arg(short, long, default_value_t = 9292)]
    port: u16,

    /// Serial port device
    #[arg(short, long, default_value_t = (& "/dev/ttyUSB0").to_string())]
    serial_port: String,

    /// Enable verbose/debug logging
    #[arg(short, long, action = clap::ArgAction::SetTrue)]
    verbose: bool,
}

async fn read_serial_port(
    serial_port_path: String
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {

    info!(
        "Opening serial port {} at 115200",
        serial_port_path
    );

    let serial_port = tokio_serial::new(serial_port_path.clone(), 115200)
        .data_bits(DataBits::Eight)
        .stop_bits(StopBits::One)
        .parity(Parity::None)
        .open_native_async()
        .map_err(|e| {
            error!("Failed to open serial port {}: {}", serial_port_path, e);
            e
        })?;

    // Larger buffer to accommodate very long P1 lines (historical entries)
    let reader = BufReader::with_capacity(32 * 1024, serial_port);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await? {
        // Only log incoming lines at debug level to avoid noisy output in normal runs
        debug!("Received message: {}", line);
        if let Err(e) = process_p1_line(&line) {
            error!("Error processing P1 line '{}': {}", line, e);
        }
    }

    warn!("Serial port stream ended; exiting.");
    Err("serial port stream ended".into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Args = Args::parse();

    // Initialize logger. If --verbose is passed, enable debug level. Otherwise default to Info.
    if args.verbose {
        // Force debug level when verbose flag is set
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Debug)
            .init();
    } else {
        // Default to Info level, but respect RUST_LOG if set
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .init();
    }

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    info!("listening on {addr}");

    let listener = TcpListener::bind(addr).await?;

    // Task that serves HTTP metrics
    let http_task = tokio::spawn(async move {
        loop {
            let (stream, _) = listener.accept().await?;

            let io = TokioIo::new(stream);

            tokio::task::spawn(async move {
                if let Err(err) = http1::Builder::new()
                    .serve_connection(io, service_fn(serve_metrics))
                    .await
                {
                    error!("Error serving connection: {:?}", err);
                }
            });
        }

        #[allow(unreachable_code)]
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    });

    // Task that reads from the serial port
    let serial_task = tokio::spawn(read_serial_port(args.serial_port));

    tokio::select! {
        res = serial_task => {
            match res {
                Ok(Ok(())) => {
                    error!("Serial task completed unexpectedly; exiting.");
                    std::process::exit(1);
                }
                Ok(Err(e)) => {
                    error!("Serial task failed: {}", e);
                    std::process::exit(1);
                }
                Err(join_err) => {
                    error!("Serial task panicked or was cancelled: {}", join_err);
                    std::process::exit(1);
                }
            }
        }
        res = http_task => {
            match res {
                Ok(Ok(())) => {
                    error!("HTTP server task completed; exiting.");
                    std::process::exit(1);
                }
                Ok(Err(e)) => {
                    error!("HTTP server task failed: {}", e);
                    std::process::exit(1);
                }
                Err(join_err) => {
                    error!("HTTP server task panicked or was cancelled: {}", join_err);
                    std::process::exit(1);
                }
            }
        }
    }
}

async fn serve_metrics(
    _: hyper::Request<hyper::body::Incoming>,
) -> Result<hyper::Response<Full<Bytes>>, Infallible> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap();

    let response = hyper::Response::builder()
        .header("Content-Type", encoder.format_type())
        .body(Full::new(Bytes::from(buffer)))
        .unwrap();

    Ok(response)
}