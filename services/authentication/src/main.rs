mod config;
mod routes;
mod services;

use lambda_http::{run, Error};
use services::Authenticator;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let authenticator = Authenticator::new().await;
    let app = routes::router(authenticator);

    run(app).await
}
