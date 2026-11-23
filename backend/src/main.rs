use api::{config::Config, telemetry};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    telemetry::init(&config.log_format);
    api::run(config).await
}
