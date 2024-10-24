use opentelemetry::global;
use opentelemetry::logs::LogError;
use opentelemetry::metrics::MetricsError;
use opentelemetry::trace::{TraceError, TracerProvider};
use opentelemetry::{
    trace::{TraceContextExt, Tracer},
    KeyValue,
};
// use once_cell::sync::Lazy;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{ExportConfig, WithExportConfig};
use opentelemetry_sdk::trace::Config;
use opentelemetry_sdk::{runtime, trace as sdktrace, Resource};
use std::error::Error;
use tracing;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;
use std::sync::LazyLock;
use log;

//TODO: READ UP ON THIS!!
static RESOURCE: LazyLock<Resource> = LazyLock::new(|| {
    Resource::new(vec![KeyValue::new(
        opentelemetry_semantic_conventions::resource::SERVICE_NAME,
        "rust-otel-demo",
    )])
});


fn init_tracer_provider() -> Result<sdktrace::TracerProvider, TraceError> {
    opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317"),
        )
        .with_trace_config(Config::default().with_resource(RESOURCE.clone()))
        .install_batch(runtime::Tokio)
}

fn init_metrics() -> Result<opentelemetry_sdk::metrics::SdkMeterProvider, MetricsError> {
    let export_config = ExportConfig {
        endpoint: "http://localhost:4317".to_string(),
        ..ExportConfig::default()
    };
    opentelemetry_otlp::new_pipeline()
        .metrics(runtime::Tokio)
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_export_config(export_config),
        )
        .with_resource(RESOURCE.clone())
        .build()
}

fn init_logs() -> Result<opentelemetry_sdk::logs::LoggerProvider, LogError> {
    opentelemetry_otlp::new_pipeline()
        .logging()
        .with_resource(RESOURCE.clone())
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint("http://localhost:4317"),
        )
        .install_batch(runtime::Tokio)
}


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {

    let result = init_tracer_provider();
    assert!(
        result.is_ok(),
        "Init tracer failed with error: {:?}",
        result.err()
    );
    let tracer_provider = result.unwrap();
    global::set_tracer_provider(tracer_provider.clone());

    let result = init_metrics();
    assert!(
        result.is_ok(),
        "Init metrics failed with error: {:?}",
        result.err()
    );
    let meter_provider = result.unwrap();
    global::set_meter_provider(meter_provider.clone());

    // Initialize logs and save the logger_provider.
    let logger_provider = init_logs().unwrap();

    // Create a new OpenTelemetryTracingBridge using the above LoggerProvider.
    let layer = OpenTelemetryTracingBridge::new(&logger_provider);
    let filter = EnvFilter::new("info")
        .add_directive("hyper=error".parse().unwrap())
        .add_directive("tonic=error".parse().unwrap())
        .add_directive("reqwest=error".parse().unwrap());

    tracing_subscriber::registry()
        .with(filter)
        .with(layer)
        .init();

    tracing::info!(name: "my-event", target: "my-target", "hello from {}. My price is {}", "apple", 1.99);
    tracing::info!(name: "my-event", target: "my-target", "hello from {}. My price is {}", "apple", 1.99);
    tracing::info!(name: "my-event", target: "my-target", "hello from {}. My price is {}", "apple", 1.99);
    log::info!("Testing regular log");
    log::error!("Testing regular error");
    

    let common_scope_attributes = vec![KeyValue::new("scope-key", "scope-value")];
    let tracer = global::tracer_provider()
        .tracer_builder("basic")
        .with_attributes(common_scope_attributes.clone())
        .build();
    let meter = global::meter_with_version(
        "basic",
        Some("v1.0"),
        Some("schema_url"),
        Some(common_scope_attributes.clone()),
    );

    let counter = meter
        .u64_counter("test_counter")
        .with_description("a simple counter for demo purposes.")
        .with_unit("my_unit")
        .init();
    for _ in 0..10 {
        counter.add(1, &[KeyValue::new("test_key", "test_value")]);
    }

    tracer.in_span("Main operation", |cx| {
        let span = cx.span();
        span.add_event(
            "Nice operation!".to_string(),
            vec![KeyValue::new("bogons", 100)],
        );
        span.set_attribute(KeyValue::new("another.key", "yes"));

        tracing::info!(name: "my-event-inside-span", target: "my-target", "hello from {}. My price is {}. I am also inside a Span!", "banana", 2.99);

        tracer.in_span("Sub operation...", |cx| {
            let span = cx.span();
            span.set_attribute(KeyValue::new("another.key", "yes"));
            span.add_event("Sub span event", vec![]);
        });
    });


    global::shutdown_tracer_provider();
    meter_provider.shutdown()?;
    logger_provider.shutdown()?;

    Ok(())
}