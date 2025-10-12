use lazy_static::lazy_static;
use prometheus::{HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Registry};

lazy_static! {
    // Global registry must come first
    pub static ref REGISTRY: Registry = Registry::new();

    // Dynamic counter with labels: actor/job type
    pub static ref COUNTER: IntCounterVec = {
        let c = IntCounterVec::new(
            prometheus::Opts::new("counter", "Total count from bootup till current time"),
            &["service", "metric_name"],
        ).unwrap();
        REGISTRY.register(Box::new(c.clone())).unwrap();
        c
    };

    // Dynamic gauge with labels: actor/session
    pub static ref ACTIVE_SESSIONS: IntGaugeVec = {
        let g = IntGaugeVec::new(
            prometheus::Opts::new("active_sessions", "Current number of active sessions"),
            &["actor"],
        ).unwrap();
        REGISTRY.register(Box::new(g.clone())).unwrap();
        g
    };

    // Dynamic histogram with labels
    pub static ref JOB_LATENCY_HISTOGRAM: HistogramVec = {
        let h = HistogramVec::new(
            HistogramOpts::new("job_latency_seconds", "Job processing latency in seconds")
                .buckets(vec![0.01, 0.05, 0.1, 0.5, 1.0, 5.0]),
            &["service", "metric_name"],
        ).unwrap();
        REGISTRY.register(Box::new(h.clone())).unwrap();
        h
    };
}
