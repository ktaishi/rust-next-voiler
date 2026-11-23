use api::config::Config;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    app_core::telemetry::init(&config.log_format);
    api::run(config).await
}
