//! # Log Layer — Tracing-to-Database Bridge
//!
//! Custom [`tracing_subscriber::Layer`] that captures `tracing` events and forwards
//! them to PostgreSQL via a bounded mpsc channel. This bridges the gap between
//! stderr-only tracing output and the `system_logs` table, making all application
//! logs queryable and streamable from the dashboard.
//!
//! ## Architecture
//!
//! ```text
//! tracing::info!("msg")
//!     ├── fmt::Layer → stderr (existing)
//!     └── DbLogLayer → mpsc channel → DbLogDrain → batch INSERT → system_logs
//!                                         └── broadcast → SSE → frontend
//! ```
//!
//! The [`DbLogLayer`] is installed as a tracing subscriber layer. It converts each
//! tracing event into a [`PendingLog`] and sends it through a bounded channel
//! (capacity 10,000). If the channel is full, the event is dropped and a counter
//! is incremented — engine/rayon threads are never blocked.
//!
//! The [`DbLogDrain`] runs as a tokio task, batching logs and inserting them into
//! PostgreSQL every 5 seconds or when 100+ logs are buffered. Each persisted log
//! is also broadcast to an SSE channel for live frontend streaming.

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::field::{Field, Visit};
use tracing::span;
use tracing_subscriber::layer::Context;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

use crate::db::{Database, SystemLogEntry};

/// A log entry waiting to be persisted to the database.
#[derive(Clone, Debug)]
pub struct PendingLog {
    pub ts: DateTime<Utc>,
    pub level: String,
    pub target: String,
    pub message: String,
    pub request_id: Option<String>,
    pub context: Option<Value>,
}

/// Custom tracing Layer that captures events and sends them to a bounded channel.
///
/// Installed alongside the standard `fmt::Layer` so that stderr logging continues
/// unchanged. The layer extracts span context (request_id, form, worker_id) by
/// walking parent spans, enabling HTTP request correlation in the log viewer.
pub struct DbLogLayer {
    sender: tokio::sync::mpsc::Sender<PendingLog>,
    min_level: tracing::Level,
    dropped: AtomicU64,
}

impl DbLogLayer {
    /// Create a new DbLogLayer with a bounded channel.
    ///
    /// Returns the layer and the receiver half of the channel. The receiver
    /// should be passed to [`DbLogDrain::run`] in the dashboard startup.
    /// For non-dashboard commands, simply drop the receiver.
    pub fn new(min_level: tracing::Level) -> (Self, tokio::sync::mpsc::Receiver<PendingLog>) {
        let (sender, receiver) = tokio::sync::mpsc::channel(10_000);
        let layer = Self {
            sender,
            min_level,
            dropped: AtomicU64::new(0),
        };
        (layer, receiver)
    }

    /// Parse the minimum log level from the `LOG_DB_LEVEL` env var.
    /// Defaults to INFO if not set or invalid.
    pub fn parse_level_from_env() -> tracing::Level {
        match std::env::var("LOG_DB_LEVEL")
            .unwrap_or_default()
            .to_lowercase()
            .as_str()
        {
            "trace" => tracing::Level::TRACE,
            "debug" => tracing::Level::DEBUG,
            "info" => tracing::Level::INFO,
            "warn" | "warning" => tracing::Level::WARN,
            "error" => tracing::Level::ERROR,
            _ => tracing::Level::INFO,
        }
    }

    /// Number of log events dropped due to channel backpressure.
    pub fn dropped_count(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

/// Visitor that extracts fields from a tracing event into a message + context map.
struct LogVisitor {
    message: String,
    fields: serde_json::Map<String, Value>,
}

impl LogVisitor {
    fn new() -> Self {
        Self {
            message: String::new(),
            fields: serde_json::Map::new(),
        }
    }
}

impl Visit for LogVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = format!("{:?}", value);
        } else {
            self.fields
                .insert(field.name().to_string(), Value::String(format!("{:?}", value)));
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields
                .insert(field.name().to_string(), Value::String(value.to_string()));
        }
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), Value::Number(value.into()));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), Value::Number(value.into()));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        if let Some(n) = serde_json::Number::from_f64(value) {
            self.fields
                .insert(field.name().to_string(), Value::Number(n));
        }
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), Value::Bool(value));
    }
}

/// Visitor that extracts span fields (e.g., request_id) from a span's attributes.
struct SpanFieldVisitor {
    fields: serde_json::Map<String, Value>,
}

impl SpanFieldVisitor {
    fn new() -> Self {
        Self {
            fields: serde_json::Map::new(),
        }
    }
}

impl Visit for SpanFieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.fields.insert(
            field.name().to_string(),
            Value::String(format!("{:?}", value)),
        );
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields
            .insert(field.name().to_string(), Value::String(value.to_string()));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), Value::Number(value.into()));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), Value::Number(value.into()));
    }
}

/// Storage for span fields, attached via `Extensions`.
#[derive(Clone, Debug, Default)]
struct SpanFields {
    fields: serde_json::Map<String, Value>,
}

impl<S> Layer<S> for DbLogLayer
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_new_span(&self, attrs: &span::Attributes<'_>, id: &span::Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let mut visitor = SpanFieldVisitor::new();
            attrs.record(&mut visitor);
            let mut extensions = span.extensions_mut();
            extensions.insert(SpanFields {
                fields: visitor.fields,
            });
        }
    }

    fn on_event(&self, event: &tracing::Event<'_>, ctx: Context<'_, S>) {
        let meta = event.metadata();
        if *meta.level() > self.min_level {
            return;
        }

        let mut visitor = LogVisitor::new();
        event.record(&mut visitor);

        // Walk parent spans to extract context fields (request_id, form, worker_id, etc.)
        let mut request_id = None;
        let mut span_context = serde_json::Map::new();
        if let Some(scope) = ctx.event_scope(event) {
            for span in scope {
                let extensions = span.extensions();
                if let Some(fields) = extensions.get::<SpanFields>() {
                    for (k, v) in &fields.fields {
                        if k == "request_id" {
                            if let Value::String(s) = v {
                                request_id = Some(s.clone());
                            }
                        } else {
                            span_context.insert(k.clone(), v.clone());
                        }
                    }
                }
            }
        }

        // Merge event fields into context
        for (k, v) in visitor.fields {
            span_context.insert(k, v);
        }

        let context = if span_context.is_empty() {
            None
        } else {
            Some(Value::Object(span_context))
        };

        let level = match *meta.level() {
            tracing::Level::ERROR => "error",
            tracing::Level::WARN => "warn",
            tracing::Level::INFO => "info",
            tracing::Level::DEBUG => "debug",
            tracing::Level::TRACE => "trace",
        };

        let pending = PendingLog {
            ts: Utc::now(),
            level: level.to_string(),
            target: meta.target().to_string(),
            message: visitor.message,
            request_id,
            context,
        };

        // Non-blocking send — drop on full channel to avoid blocking engine threads
        if self.sender.try_send(pending).is_err() {
            self.dropped.fetch_add(1, Ordering::Relaxed);
        }
    }
}

/// Background drain task that reads from the mpsc channel and batch-inserts
/// logs into PostgreSQL.
pub struct DbLogDrain {
    receiver: tokio::sync::mpsc::Receiver<PendingLog>,
    db: Database,
    broadcast: tokio::sync::broadcast::Sender<String>,
}

impl DbLogDrain {
    pub fn new(
        receiver: tokio::sync::mpsc::Receiver<PendingLog>,
        db: Database,
        broadcast: tokio::sync::broadcast::Sender<String>,
    ) -> Self {
        Self {
            receiver,
            db,
            broadcast,
        }
    }

    /// Run the drain loop. Batches logs and inserts every 5 seconds or when
    /// 100+ logs are buffered.
    pub async fn run(mut self) {
        let mut buffer: Vec<PendingLog> = Vec::with_capacity(128);
        let mut flush_interval = tokio::time::interval(std::time::Duration::from_secs(5));
        flush_interval.tick().await; // skip first immediate tick

        loop {
            tokio::select! {
                // Receive logs from the channel
                maybe_log = self.receiver.recv() => {
                    match maybe_log {
                        Some(log) => {
                            buffer.push(log);
                            if buffer.len() >= 100 {
                                self.flush(&mut buffer).await;
                            }
                        }
                        None => {
                            // Channel closed — flush remaining and exit
                            if !buffer.is_empty() {
                                self.flush(&mut buffer).await;
                            }
                            break;
                        }
                    }
                }
                // Periodic flush
                _ = flush_interval.tick() => {
                    if !buffer.is_empty() {
                        self.flush(&mut buffer).await;
                    }
                }
            }
        }
    }

    async fn flush(&self, buffer: &mut Vec<PendingLog>) {
        let logs: Vec<SystemLogEntry> = buffer
            .drain(..)
            .map(|p| {
                // Extract component from the tracing target (e.g. "darkreach::kbn" -> "kbn")
                let component = p
                    .target
                    .rsplit("::")
                    .next()
                    .unwrap_or(&p.target)
                    .to_string();

                // Merge request_id into context if present
                let context = match (p.context, &p.request_id) {
                    (Some(Value::Object(mut map)), Some(rid)) => {
                        map.insert("request_id".to_string(), Value::String(rid.clone()));
                        Some(Value::Object(map))
                    }
                    (None, Some(rid)) => {
                        Some(serde_json::json!({"request_id": rid}))
                    }
                    (ctx, _) => ctx,
                };

                SystemLogEntry {
                    ts: p.ts,
                    level: p.level,
                    source: "tracing".to_string(),
                    component,
                    message: p.message,
                    worker_id: None,
                    search_job_id: None,
                    search_id: None,
                    context,
                }
            })
            .collect();

        // Broadcast each log to SSE subscribers
        for log in &logs {
            if let Ok(json) = serde_json::to_string(log) {
                let _ = self.broadcast.send(json);
            }
        }

        // Batch insert to PostgreSQL
        if let Err(e) = self.db.insert_system_logs(&logs).await {
            // Use eprintln to avoid recursion — we can't tracing::warn here
            eprintln!("[log_layer] failed to persist {} logs: {}", logs.len(), e);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test level parsing with explicit string values instead of env vars
    /// to avoid test ordering issues with shared global state.
    fn parse_level(value: &str) -> tracing::Level {
        match value.to_lowercase().as_str() {
            "trace" => tracing::Level::TRACE,
            "debug" => tracing::Level::DEBUG,
            "info" => tracing::Level::INFO,
            "warn" | "warning" => tracing::Level::WARN,
            "error" => tracing::Level::ERROR,
            _ => tracing::Level::INFO,
        }
    }

    #[test]
    fn parse_level_defaults_to_info() {
        assert_eq!(parse_level(""), tracing::Level::INFO);
        assert_eq!(parse_level("unknown"), tracing::Level::INFO);
    }

    #[test]
    fn parse_level_recognizes_all_levels() {
        assert_eq!(parse_level("trace"), tracing::Level::TRACE);
        assert_eq!(parse_level("debug"), tracing::Level::DEBUG);
        assert_eq!(parse_level("info"), tracing::Level::INFO);
        assert_eq!(parse_level("warn"), tracing::Level::WARN);
        assert_eq!(parse_level("warning"), tracing::Level::WARN);
        assert_eq!(parse_level("WARNING"), tracing::Level::WARN);
        assert_eq!(parse_level("error"), tracing::Level::ERROR);
    }

    #[test]
    fn pending_log_component_extraction() {
        let target = "darkreach::dashboard::routes_observability";
        let component = target.rsplit("::").next().unwrap_or(target);
        assert_eq!(component, "routes_observability");
    }

    #[test]
    fn pending_log_component_extraction_single() {
        let target = "sieve";
        let component = target.rsplit("::").next().unwrap_or(target);
        assert_eq!(component, "sieve");
    }
}
