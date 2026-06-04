//! Metrics: counters, gauges, and request statistics.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

/// A single metric sample.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sample {
    /// Metric name.
    pub name: String,
    /// Numeric value.
    pub value: f64,
    /// Optional labels.
    pub labels: HashMap<String, String>,
}

/// Atomic counter.
#[derive(Debug, Default)]
pub struct Counter {
    inner: parking_lot::Mutex<f64>,
}

impl Counter {
    /// Construct a new counter at 0.
    pub fn new() -> Self {
        Self {
            inner: parking_lot::Mutex::new(0.0),
        }
    }
    /// Increment by `n`.
    pub fn inc(&self, n: f64) {
        *self.inner.lock() += n;
    }
    /// Increment by 1.
    pub fn inc1(&self) {
        self.inc(1.0);
    }
    /// Read the current value.
    pub fn get(&self) -> f64 {
        *self.inner.lock()
    }
    /// Reset to 0.
    pub fn reset(&self) {
        *self.inner.lock() = 0.0;
    }
}

/// A gauge value.
#[derive(Debug, Default)]
pub struct Gauge {
    inner: parking_lot::Mutex<f64>,
}

impl Gauge {
    /// Construct a new gauge at `initial`.
    pub fn new(initial: f64) -> Self {
        Self {
            inner: parking_lot::Mutex::new(initial),
        }
    }
    /// Set the value.
    pub fn set(&self, v: f64) {
        *self.inner.lock() = v;
    }
    /// Read the value.
    pub fn get(&self) -> f64 {
        *self.inner.lock()
    }
}

/// Histogram (count + sum + min + max + mean).
#[derive(Debug, Default)]
pub struct Histogram {
    inner: parking_lot::Mutex<HistogramData>,
}

/// A point-in-time snapshot of a histogram.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistogramData {
    /// Number of samples.
    pub count: u64,
    /// Sum of samples.
    pub sum: f64,
    /// Minimum sample.
    pub min: f64,
    /// Maximum sample.
    pub max: f64,
    /// Last sample.
    pub last: f64,
}

impl Histogram {
    /// Construct a new histogram.
    pub fn new() -> Self {
        Self {
            inner: parking_lot::Mutex::new(HistogramData::default()),
        }
    }
    /// Record a sample.
    pub fn observe(&self, v: f64) {
        let mut g = self.inner.lock();
        g.count += 1;
        g.sum += v;
        g.last = v;
        if g.count == 1 {
            g.min = v;
            g.max = v;
        } else {
            g.min = g.min.min(v);
            g.max = g.max.max(v);
        }
    }
    /// Take a snapshot.
    pub fn snapshot(&self) -> HistogramData {
        self.inner.lock().clone()
    }
    /// Mean of all observed values.
    pub fn mean(&self) -> f64 {
        let g = self.inner.lock();
        if g.count == 0 {
            0.0
        } else {
            g.sum / g.count as f64
        }
    }
}

/// Aggregate proxy metrics.
#[derive(Debug, Default)]
pub struct Metrics {
    /// Total requests received.
    pub requests_total: Counter,
    /// Number of successful (2xx/3xx) responses.
    pub responses_2xx_3xx: Counter,
    /// Number of 4xx responses.
    pub responses_4xx: Counter,
    /// Number of 5xx responses.
    pub responses_5xx: Counter,
    /// Currently in-flight requests.
    pub in_flight: Gauge,
    /// Number of TLS handshakes.
    pub tls_handshakes: Counter,
    /// Request duration histogram.
    pub request_duration: Histogram,
    /// Upstream connect duration histogram.
    pub upstream_connect_duration: Histogram,
    /// Registered routes gauge.
    pub routes: Gauge,
    /// Start time.
    pub started_at: RwLock<Option<Instant>>,
}

impl Metrics {
    /// Construct a new metrics set, recording the start time.
    pub fn new() -> Arc<Self> {
        let m = Arc::new(Self::default());
        *m.started_at.write() = Some(Instant::now());
        m
    }

    /// Take a snapshot of all metrics.
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            requests_total: self.requests_total.get(),
            responses_2xx_3xx: self.responses_2xx_3xx.get(),
            responses_4xx: self.responses_4xx.get(),
            responses_5xx: self.responses_5xx.get(),
            in_flight: self.in_flight.get(),
            tls_handshakes: self.tls_handshakes.get(),
            request_duration: self.request_duration.snapshot(),
            upstream_connect_duration: self.upstream_connect_duration.snapshot(),
            routes: self.routes.get(),
            uptime_secs: self
                .started_at
                .read()
                .map(|t| t.elapsed().as_secs_f64())
                .unwrap_or(0.0),
        }
    }

    /// Render as Prometheus-style text.
    pub fn render_prometheus(&self) -> String {
        let s = self.snapshot();
        let mut out = String::new();
        out.push_str("# HELP portless_requests_total Total number of requests\n");
        out.push_str("# TYPE portless_requests_total counter\n");
        out.push_str(&format!("portless_requests_total {}\n", s.requests_total));
        out.push_str("# HELP portless_responses_2xx_3xx Successful responses\n");
        out.push_str("# TYPE portless_responses_2xx_3xx counter\n");
        out.push_str(&format!(
            "portless_responses_2xx_3xx {}\n",
            s.responses_2xx_3xx
        ));
        out.push_str("# HELP portless_responses_4xx Client error responses\n");
        out.push_str("# TYPE portless_responses_4xx counter\n");
        out.push_str(&format!("portless_responses_4xx {}\n", s.responses_4xx));
        out.push_str("# HELP portless_responses_5xx Server error responses\n");
        out.push_str("# TYPE portless_responses_5xx counter\n");
        out.push_str(&format!("portless_responses_5xx {}\n", s.responses_5xx));
        out.push_str("# HELP portless_tls_handshakes_total TLS handshakes\n");
        out.push_str("# TYPE portless_tls_handshakes_total counter\n");
        out.push_str(&format!(
            "portless_tls_handshakes_total {}\n",
            s.tls_handshakes
        ));
        out.push_str("# HELP portless_in_flight In-flight requests\n");
        out.push_str("# TYPE portless_in_flight gauge\n");
        out.push_str(&format!("portless_in_flight {}\n", s.in_flight));
        out.push_str("# HELP portless_routes Registered routes\n");
        out.push_str("# TYPE portless_routes gauge\n");
        out.push_str(&format!("portless_routes {}\n", s.routes));
        out.push_str("# HELP portless_request_duration_seconds Request duration\n");
        out.push_str("# TYPE portless_request_duration_seconds histogram\n");
        out.push_str(&format!(
            "portless_request_duration_seconds_count {}\nportless_request_duration_seconds_sum {}\n",
            s.request_duration.count, s.request_duration.sum
        ));
        out
    }
}

/// A point-in-time snapshot of [`Metrics`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Total requests.
    pub requests_total: f64,
    /// 2xx/3xx count.
    pub responses_2xx_3xx: f64,
    /// 4xx count.
    pub responses_4xx: f64,
    /// 5xx count.
    pub responses_5xx: f64,
    /// In-flight gauge.
    pub in_flight: f64,
    /// TLS handshakes.
    pub tls_handshakes: f64,
    /// Request duration histogram.
    pub request_duration: HistogramData,
    /// Upstream connect duration histogram.
    pub upstream_connect_duration: HistogramData,
    /// Registered routes.
    pub routes: f64,
    /// Process uptime in seconds.
    pub uptime_secs: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counter_basic() {
        let c = Counter::new();
        c.inc1();
        c.inc1();
        assert_eq!(c.get(), 2.0);
    }

    #[test]
    fn histogram_basic() {
        let h = Histogram::new();
        h.observe(0.1);
        h.observe(0.2);
        h.observe(0.5);
        let s = h.snapshot();
        assert_eq!(s.count, 3);
        assert!((s.sum - 0.8).abs() < 1e-9);
        assert!((s.min - 0.1).abs() < 1e-9);
        assert!((s.max - 0.5).abs() < 1e-9);
    }

    #[test]
    fn prometheus_render() {
        let m = Metrics::new();
        m.requests_total.inc1();
        let s = m.render_prometheus();
        assert!(s.contains("portless_requests_total"));
    }
}
