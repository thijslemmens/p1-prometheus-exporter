# Kamstrup 162JXC P1 Prometheus exporter

This is a Prometheus exporter
for [Kamstrup 162JXC](https://www.kamstrup.com/nl-nl/elektriciteitsoplossingen/slimme-elektriciteitsmeters/previous-products/kamstrup-162j)
smart meters in the Netherlands. It also retrieves the gas meter readings.

Requires a P1 cable connected to your computer, I
used [this one](https://webshop.cedel.nl/nl/Slimme-meter-kabel-P1-naar-USB).

## Building

1. Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
2. Run: `cargo run` or build for release `cargo build --release`

### Debian package

To build a Debian package:

```
$ cargo install cargo-deb
$ cargo deb
```

## Implementation Details

The P1 port carries is a pretty simple protocol, called DSMR (Dutch Smart Meter Requirements). On each line it provides
an address for a metric and the value of that metric. Every ten seconds you get a new reading over the serial line. An
explanation of what each address means can be found in the https://github.com/energietransitie/dsmr-info project.

The Kamstrup 162JXC adheres to DSMR 3.0. This project will probably also work with other DSMR 3.0 meters.

Below is a typical message:

```
/KMP5 KA6UXXXXXXXXXXXX
0-0:96.1.1(XXXXXX)
1-0:1.8.1(20203.975*kWh)
1-0:1.8.2(18247.900*kWh)
1-0:2.8.1(00238.368*kWh)
1-0:2.8.2(00532.631*kWh)
0-0:96.14.0(0001)
1-0:1.7.0(0000.29*kW)
1-0:2.7.0(0000.00*kW)
0-0:96.13.1()
0-0:96.13.0()
0-1:24.1.0(3)
0-1:96.1.0(XXXXXX)
0-1:24.3.0(240914210000)(08)(60)(1)(0-1:24.2.1)(m3)
(14094.865)
!
```

This results in the following Prometheus export:

```
# HELP kamstrup_162jxc_p1_actual_power_delivered_watts Actual electricity power delivered (+P) in 1 Watt resolution (1-0:1.7.0)
# TYPE kamstrup_162jxc_p1_actual_power_delivered_watts gauge
kamstrup_162jxc_p1_actual_power_delivered_watts 290
# HELP kamstrup_162jxc_p1_actual_power_received_watts Actual electricity power received (-P) in 1 Watt resolution (1-0:2.7.0)
# TYPE kamstrup_162jxc_p1_actual_power_received_watts gauge
kamstrup_162jxc_p1_actual_power_received_watts 0
# HELP kamstrup_162jxc_p1_total_energy_delivered_watthours Total electricity energy delivered (+P) in 1 Watthour resolution (1-0:1.8.1 / 1-0:1.8.2)
# TYPE kamstrup_162jxc_p1_total_energy_delivered_watthours counter
kamstrup_162jxc_p1_total_energy_delivered_watthours{meter="1"} 20203975
kamstrup_162jxc_p1_total_energy_delivered_watthours{meter="2"} 18247900
# HELP kamstrup_162jxc_p1_total_energy_received_watthours Total electricity energy received (-P) in 1 Watthour resolution (1-0:2.8.1 / 1-0:2.8.2)
# TYPE kamstrup_162jxc_p1_total_energy_received_watthours counter
kamstrup_162jxc_p1_total_energy_received_watthours{meter="1"} 238368
kamstrup_162jxc_p1_total_energy_received_watthours{meter="2"} 532631
# HELP kamstrup_162jxc_p1_total_gas_delivered_m3 Total gas delivered in m3 (0-1:24.3.0)
# TYPE kamstrup_162jxc_p1_total_gas_delivered_m3 counter
kamstrup_162jxc_p1_total_gas_delivered_m3 14094.865
```
