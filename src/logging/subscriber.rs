use tracing::subscriber::set_global_default;
use tracing::{error, trace, Subscriber};
use tracing_bunyan_formatter::JsonStorageLayer;
use tracing_subscriber::fmt;
use tracing_subscriber::{EnvFilter, Registry, fmt::MakeWriter};
use tracing_subscriber::prelude::__tracing_subscriber_SubscriberExt;

pub fn get_subscribers<S, W>(_name: S, level: S, writer: W) -> impl Subscriber
where
    S: Into<String>,
    W: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let severity_filter_layer =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level.into()));

    let formatting_layer = fmt::layer()
        .with_writer(writer)
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true);

    Registry::default()
        .with(severity_filter_layer)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    match set_global_default(subscriber) {
        Ok(_) => trace!("Initialized trace subscriber"),
        Err(err) => error!("Failed to initalize trace subscriber - {:?}", err)
    }
}