use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init(format: &str) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,api=debug,tower_http=info,sqlx=warn"));
    let registry = tracing_subscriber::registry().with(filter);
    if format == "json" {
        registry.with(fmt::layer().json()).init();
    } else {
        registry.with(fmt::layer()).init();
    }
}
