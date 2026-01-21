use prometheus::{
    register_counter, register_int_counter_vec, register_int_gauge, Counter, Gauge, IntCounterVec,
    IntGauge,
};
use std::collections::HashMap;

/// How to interpret and export one OBIS value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum MetricKind {
    GaugeI64,
    // GaugeF64,
    IntCounterDelta, // monotonically increasing integer (delta logic)
    CounterDeltaF64, // monotonically increasing float (delta logic)
}

#[derive(Clone, Copy)]
struct MetricConfig {
    obis: &'static str,
    name: &'static str,
    help: &'static str,
    kind: MetricKind,
    scale: f64,
    labels: &'static [&'static str], // label values
}

static METRIC_CONFIG: &[MetricConfig] = &[
    MetricConfig {
        obis: "1-0:1.7.0",
        name: "p1_actual_power_delivered_watts",
        help: "Actual electricity power delivered (+P) in watts (1-0:1.7.0)",
        kind: MetricKind::GaugeI64,
        scale: 1000.0, // kW -> W
        labels: &[],
    },
    MetricConfig {
        obis: "1-0:2.7.0",
        name: "p1_actual_power_received_watts",
        help: "Actual electricity power received (-P) in watts (1-0:2.7.0)",
        kind: MetricKind::GaugeI64,
        scale: 1000.0,
        labels: &[],
    },
    MetricConfig {
        obis: "1-0:1.8.1",
        name: "p1_total_energy_delivered_watthours",
        help: "Total electricity energy delivered (+P), tariff 1, in Wh (1-0:1.8.1)",
        kind: MetricKind::IntCounterDelta,
        scale: 1000.0,  // kWh -> Wh
        labels: &["1"], // meter = "1"
    },
    MetricConfig {
        obis: "1-0:1.8.2",
        name: "p1_total_energy_delivered_watthours",
        help: "Total electricity energy delivered (+P), tariff 2, in Wh (1-0:1.8.2)",
        kind: MetricKind::IntCounterDelta,
        scale: 1000.0,
        labels: &["2"],
    },
    MetricConfig {
        obis: "1-0:2.8.1",
        name: "p1_total_energy_received_watthours",
        help: "Total electricity energy received (-P), tariff 1, in Wh (1-0:2.8.1)",
        kind: MetricKind::IntCounterDelta,
        scale: 1000.0,
        labels: &["1"],
    },
    MetricConfig {
        obis: "1-0:2.8.2",
        name: "p1_total_energy_received_watthours",
        help: "Total electricity energy received (-P), tariff 2, in Wh (1-0:2.8.2)",
        kind: MetricKind::IntCounterDelta,
        scale: 1000.0,
        labels: &["2"],
    },
    MetricConfig {
        obis: "0-1:24.2.3",
        name: "p1_total_gas_delivered_m3",
        help: "Total gas delivered in m3 (0-1:24.2.3)",
        kind: MetricKind::CounterDeltaF64,
        scale: 1.0,
        labels: &[],
    },
    // You can add more metrics here without touching process_p1_line.
];

/// Runtime handle: per OBIS we know which metric instance + labels to use.
struct MetricRuntime {
    kind: MetricKind,
    scale: f64,
    gauge_i64: Option<IntGauge>,
    gauge_f64: Option<Gauge>,
    int_counter_vec: Option<IntCounterVec>,
    counter_f64: Option<Counter>,
    labels: &'static [&'static str],
}

lazy_static::lazy_static! {
    static ref RUNTIME_BY_OBIS: HashMap<&'static str, MetricRuntime> = {
        // 1. Register metrics once per unique (name, kind)
        #[derive(Hash, Eq, PartialEq, Debug)]
        struct Key {
            name: &'static str,
            kind: MetricKind,
        }

        let mut metric_by_name: HashMap<Key, MetricRuntime> = HashMap::new();

        for cfg in METRIC_CONFIG {
            let key = Key { name: cfg.name, kind: cfg.kind };

            // If not yet registered, register now
            metric_by_name.entry(key).or_insert_with(|| {
                match cfg.kind {
                    MetricKind::GaugeI64 => {
                        let g = register_int_gauge!(cfg.name, cfg.help)
                            .expect("failed to register IntGauge");
                        MetricRuntime {
                            kind: cfg.kind,
                            scale: cfg.scale,
                            gauge_i64: Some(g),
                            gauge_f64: None,
                            int_counter_vec: None,
                            counter_f64: None,
                            labels: &[],
                        }
                    }
                    // MetricKind::GaugeF64 => {
                    //     let g = register_gauge!(cfg.name, cfg.help)
                    //         .expect("failed to register Gauge");
                    //     MetricRuntime {
                    //         kind: cfg.kind,
                    //         scale: cfg.scale,
                    //         gauge_i64: None,
                    //         gauge_f64: Some(g),
                    //         int_counter_vec: None,
                    //         counter_f64: None,
                    //         labels: &[],
                    //     }
                    // }
                    MetricKind::IntCounterDelta => {
                        // Here we fix the label name to "meter" for all such metrics.
                        let c = register_int_counter_vec!(
                            cfg.name,
                            cfg.help,
                            &["tariff"]
                        ).expect("failed to register IntCounterVec");
                        MetricRuntime {
                            kind: cfg.kind,
                            scale: cfg.scale,
                            gauge_i64: None,
                            gauge_f64: None,
                            int_counter_vec: Some(c),
                            counter_f64: None,
                            labels: &[],
                        }
                    }
                    MetricKind::CounterDeltaF64 => {
                        let c = register_counter!(cfg.name, cfg.help)
                            .expect("failed to register Counter");
                        MetricRuntime {
                            kind: cfg.kind,
                            scale: cfg.scale,
                            gauge_i64: None,
                            gauge_f64: None,
                            int_counter_vec: None,
                            counter_f64: Some(c),
                            labels: &[],
                        }
                    }
                }
            });
        }

        // 2. Build RUNTIME_BY_OBIS using the per-name metric handles
        let mut by_obis: HashMap<&'static str, MetricRuntime> = HashMap::new();

        for cfg in METRIC_CONFIG {
            let key = Key { name: cfg.name, kind: cfg.kind };
            let base = metric_by_name.get(&key).expect("metric not registered");

            // Clone the runtime and override scale + labels per OBIS
            let runtime = MetricRuntime {
                kind: base.kind,
                scale: cfg.scale,
                gauge_i64: base.gauge_i64.clone(),
                gauge_f64: base.gauge_f64.clone(),
                int_counter_vec: base.int_counter_vec.clone(),
                counter_f64: base.counter_f64.clone(),
                labels: cfg.labels,
            };

            by_obis.insert(cfg.obis, runtime);
        }

        by_obis
    };
}

#[derive(Debug)]
struct ParsedP1Line<'a> {
    obis: &'a str,
    raw_value: &'a str,
}

fn parse_p1_line(line: &str) -> Option<ParsedP1Line<'_>> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('!') {
        return None;
    }

    let obis_end = line.find('(')?;
    let obis = &line[..obis_end];

    // Look up config for this OBIS, if any
    let cfg = METRIC_CONFIG.iter().find(|c| c.obis == obis)?;

    // Helper: extract numeric prefix from a string like "006495.01000000W"
    fn extract_numeric_prefix(s: &str) -> Option<String> {
        let mut out = String::new();
        for ch in s.chars() {
            if ch.is_ascii_digit() || ch == '.' {
                out.push(ch);
            } else {
                break;
            }
        }
        if out.is_empty() {
            None
        } else {
            Some(out)
        }
    }

    if cfg.kind == MetricKind::IntCounterDelta {
        // For energy counters, take the FIRST "(...)" and grab numeric prefix
        let first_open = line.find('(')?;
        let first_close = line[first_open..].find(')')? + first_open;
        let inner = &line[first_open + 1..first_close]; // something like "006495.01000000W" or "006495.662*kWh"

        // If there's a '*', split before it
        let inner_val = inner.split('*').next().unwrap_or(inner).trim();

        let numeric = extract_numeric_prefix(inner_val)?;
        // Verify it parses
        if numeric.parse::<f64>().is_err() {
            return None;
        }

        return Some(ParsedP1Line {
            obis,
            // store a &str; numeric is a String, so keep inner_val and assume it starts numeric
            raw_value: inner_val
                .split(|c: char| !c.is_ascii_digit() && c != '.')
                .next()
                .unwrap_or(inner_val),
        });
    }

    // Default behavior: use the LAST "(...)" block and expect value[*unit]
    let last_open = line.rfind('(')?;
    let last_close = line[last_open..].find(')')? + last_open;
    let inner = &line[last_open + 1..last_close];

    let mut parts = inner.split('*');
    let raw_value = parts.next()?.trim();

    if raw_value.parse::<f64>().is_err() {
        return None;
    }

    Some(ParsedP1Line { obis, raw_value })
}

pub fn process_p1_line(line: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let parsed = match parse_p1_line(line) {
        Some(p) => p,
        None => return Ok(()),
    };

    let runtime = match RUNTIME_BY_OBIS.get(parsed.obis) {
        Some(r) => r,
        None => return Ok(()), // unknown OBIS, ignore
    };

    let value: f64 = parsed.raw_value.parse()?;
    let scaled = value * runtime.scale;

    match runtime.kind {
        MetricKind::GaugeI64 => {
            let g = runtime.gauge_i64.as_ref().unwrap();
            g.set(scaled.round() as i64);
        }
        // MetricKind::GaugeF64 => {
        //     let g = runtime.gauge_f64.as_ref().unwrap();
        //     g.set(scaled);
        // }
        MetricKind::IntCounterDelta => {
            let cvec = runtime.int_counter_vec.as_ref().unwrap();
            // For now assume one label: "meter".
            let labels = runtime.labels;
            let c = cvec.with_label_values(labels);
            let prev = cvec.get_metric_with_label_values(labels)?.get();
            let curr = scaled.round() as u64;
            if curr > prev {
                c.inc_by(curr - prev);
            }
        }
        MetricKind::CounterDeltaF64 => {
            let c = runtime.counter_f64.as_ref().unwrap();
            let prev = c.get();
            let curr = scaled;
            if curr > prev {
                c.inc_by(curr - prev);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use prometheus::{gather, proto::MetricType};

    fn get_int_gauge_value(metric_name: &str) -> Option<i64> {
        for mf in gather() {
            if mf.name() == metric_name && mf.get_field_type() == MetricType::GAUGE {
                for m in mf.get_metric() {
                    return Some(m.gauge.value.unwrap_or(0.0) as i64);
                }
            }
        }
        None
    }

    fn get_int_counter_vec_value(metric_name: &str, label: (&str, &str)) -> Option<u64> {
        let (label_name, label_value) = label;
        'outer: for mf in gather() {
            if mf.name() == metric_name && mf.get_field_type() == MetricType::COUNTER {
                for m in mf.get_metric() {
                    let mut ok = false;
                    for l in m.get_label() {
                        if l.name.as_deref() == Some(label_name) && l.value.as_deref() == Some(label_value) {
                            ok = true;
                            break;
                        }
                    }
                    if ok {
                        return Some(m.counter.value.unwrap_or(0.0) as u64);
                    } else {
                        continue 'outer;
                    }
                }
            }
        }
        None
    }

    #[test]
    fn parses_current_power_consumed() {
        // 1.230 kW -> 1230 W, metric name from METRIC_CONFIG for obis "1-0:1.7.0"
        process_p1_line("1-0:1.7.0(01.230*kW)").unwrap();

        let v = get_int_gauge_value("p1_actual_power_delivered_watts").expect("gauge not found");
        assert_eq!(v, 1230);
    }

    #[test]
    fn parses_total_energy_consumed_tariff_1() {
        // Metric name from METRIC_CONFIG for obis "1-0:1.8.1"
        let metric_name = "p1_total_energy_delivered_watthours";
        let before = get_int_counter_vec_value(metric_name, ("tariff", "1")).unwrap_or(0);

        // From your sample: 006495.662 kWh -> 6_495_662 Wh
        process_p1_line("1-0:1.8.1(006495.662*kWh)").unwrap();

        let after =
            get_int_counter_vec_value(metric_name, ("tariff", "1")).expect("counter not found");
        assert_eq!(after - before, 6_495_662);
    }

    #[test]
    fn parses_gas_from_extended_line() {
        let metric_name = "p1_total_gas_delivered_m3";

        // Read before
        let before = {
            let mut val = 0.0;
            for mf in gather() {
                if mf.name() == metric_name {
                    for m in mf.get_metric() {
                        val = m.counter.value.unwrap_or(0.0);
                    }
                }
            }
            val
        };

        // 0-1:24.2.3(260111190500W)(05890.569*m3)
        process_p1_line("0-1:24.2.3(260111190500W)(05890.569*m3)").unwrap();

        let after = {
            let mut val = 0.0;
            for mf in gather() {
                if mf.name() == metric_name {
                    for m in mf.get_metric() {
                        val = m.counter.value.unwrap_or(0.0);
                    }
                }
            }
            val
        };

        // CounterDeltaF64 logic: from 0 to 5890.569
        assert!((after - before - 5890.569_f64).abs() < 0.0001);
    }
}
