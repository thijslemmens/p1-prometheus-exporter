use std::convert::Infallible;
use std::net::SocketAddr;

use clap::Parser;
use http_body_util::Full;
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use log::info;
use prometheus::{Counter, Encoder, Gauge, register_counter, register_gauge, TextEncoder, register_counter_vec, CounterVec};
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
}


// Initialize a counter metric
lazy_static::lazy_static! {
    static ref CURRENT_POWER_CONSUMED: Gauge = register_gauge!(
        "kamstrup_162jxc_p1_actual_power_delivered_watts",
        "Actual electricity power delivered (+P) in 1 Watt resolution (1-0:1.7.0)"
    ).unwrap();
    static ref CURRENT_POWER_PRODUCED: Gauge = register_gauge!(
        "kamstrup_162jxc_p1_actual_power_received_watts",
        "Actual electricity power received (-P) in 1 Watt resolution (1-0:2.7.0)"
    ).unwrap();

    static ref TOTAL_ENERGY_CONSUMED: CounterVec = register_counter_vec!(
        "kamstrup_162jxc_p1_total_energy_delivered_watthours",
        "Total electricity energy delivered (+P) in 1 Watthour resolution (1-0:1.8.1 / 1-0:1.8.2)",
        &["meter"]
    ).unwrap();
    static ref TOTAL_ENERGY_PRODUCED: CounterVec = register_counter_vec!(
        "kamstrup_162jxc_p1_total_energy_received_watthours",
        "Total electricity energy received (-P) in 1 Watthour resolution (1-0:2.8.1 / 1-0:2.8.2)",
        &["meter"]
    ).unwrap();
    static ref TOTAL_GAS_CONSUMED: Counter = register_counter!(
        "kamstrup_162jxc_p1_total_gas_delivered_m3",
        "Total gas delivered in 1 m3 resolution (0-1:24.3.0)"
    ).unwrap();
}


async fn read_serial_port(serial_port_path: String) {
    // Open the serial port (update with your port and baud rate)
    let serial_port = tokio_serial::new(serial_port_path, 9600)
        .data_bits(DataBits::Seven)
        .stop_bits(StopBits::One)
        .parity(Parity::Even)
        .open_native_async().unwrap();

    // Buffer the serial stream for efficient reading
    let reader = BufReader::new(serial_port);
    let mut lines = reader.lines();

    while let Some(line) = lines.next_line().await.unwrap() {
        println!("Received message: {}", line);

        if line.starts_with("(") {
            let curr = line[1..line.len() - 1].parse::<f64>().unwrap();
            println!("TOTAL_GAS_CONSUMED: {}", curr);
            TOTAL_GAS_CONSUMED.inc_by(curr - TOTAL_GAS_CONSUMED.get())
        } else if line.starts_with("1-0:1.8.1") {
            let curr = line[10..line.len() - 5].parse::<f64>().unwrap() * 1000.0;
            let meter = "1";
            println!("TOTAL_ENERGY_CONSUMED: {}, meter: {}", curr, meter);
            TOTAL_ENERGY_CONSUMED.with_label_values(&[meter]).inc_by(curr - TOTAL_ENERGY_CONSUMED.get_metric_with_label_values(&[meter]).unwrap().get())
        } else if line.starts_with("1-0:1.8.2") {
            let curr = line[10..line.len() - 5].parse::<f64>().unwrap() * 1000.0;
            let meter = "2";
            println!("TOTAL_ENERGY_CONSUMED: {}, meter: {}", curr, meter);
            TOTAL_ENERGY_CONSUMED.with_label_values(&[meter]).inc_by(curr - TOTAL_ENERGY_CONSUMED.get_metric_with_label_values(&[meter]).unwrap().get())
        } else if line.starts_with("1-0:2.8.1") {
            let curr = line[10..line.len() - 5].parse::<f64>().unwrap() * 1000.0;
            let meter = "1";
            println!("TOTAL_ENERGY_PRODUCED: {}, meter: {}", curr, meter);
            TOTAL_ENERGY_PRODUCED.with_label_values(&[meter]).inc_by(curr - TOTAL_ENERGY_PRODUCED.get_metric_with_label_values(&[meter]).unwrap().get())
        } else if line.starts_with("1-0:2.8.2") {
            let curr = line[10..line.len() - 5].parse::<f64>().unwrap() * 1000.0;
            let meter = "2";
            println!("TOTAL_ENERGY_PRODUCED: {}, meter: {}", curr, meter);
            TOTAL_ENERGY_PRODUCED.with_label_values(&[meter]).inc_by(curr - TOTAL_ENERGY_PRODUCED.get_metric_with_label_values(&[meter]).unwrap().get())
        } else if line.starts_with("1-0:1.7.0") {
            let curr = line[10..line.len() - 5].parse::<f64>().unwrap() * 1000.0;
            println!("CURRENT_POWER_CONSUMED: {}", curr);
            CURRENT_POWER_CONSUMED.set(curr);
        } else if line.starts_with("1-0:2.7.0") {
            let curr = line[10..line.len() - 5].parse::<f64>().unwrap() * 1000.0;
            println!("CURRENT_POWER_PRODUCED: {}", curr);
            CURRENT_POWER_PRODUCED.set(curr);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Args = Args::parse();

    tokio::spawn(read_serial_port(args.serial_port));

    // Define the address and create the server
    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    info!("listening on {addr}");

    // We create a TcpListener and bind it to 127.0.0.1:3000
    let listener = TcpListener::bind(addr).await?;

    // We start a loop to continuously accept incoming connections
    loop {
        let (stream, _) = listener.accept().await?;

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);

        // Spawn a tokio task to serve multiple connections concurrently
        tokio::task::spawn(async move {
            // Finally, we bind the incoming connection to our `hello` service
            if let Err(err) = http1::Builder::new()
                // `service_fn` converts our function in a `Service`
                .serve_connection(io, service_fn(serve_metrics))
                .await
            {
                eprintln!("Error serving connection: {:?}", err);
            }
        });
    }
}

async fn serve_metrics(_: hyper::Request<hyper::body::Incoming>) -> Result<hyper::Response<Full<Bytes>>, Infallible> {
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

