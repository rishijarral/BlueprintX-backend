use crate::config::Environment;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn init_logging(env: &Environment) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        // Default log levels based on environment
        match env {
            Environment::Dev => "blueprintx_backend=debug,tower_http=debug,info".into(),
            Environment::Staging => "blueprintx_backend=debug,tower_http=info,info".into(),
            Environment::Prod => "blueprintx_backend=info,tower_http=info,warn".into(),
        }
    });

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_file(env.is_dev())
        .with_line_number(env.is_dev());

    // Use JSON format in production, pretty format in dev
    if matches!(env, Environment::Prod) {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer.json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer.pretty())
            .init();
    }

    tracing::info!("Logging initialized for {:?} environment", env);
}
