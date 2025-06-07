use axum::{
    middleware::from_fn,
    routing::{get, post},
    Router,
};
use std::net::SocketAddr;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

pub mod config;
mod middleware;
mod models;
mod routes;
mod utils;

use self::config::Config;
use self::middleware::{AppState, logging_middleware};
use self::routes::{create, lookup};
pub use self::utils::generate_code;

fn create_router(app_state: AppState) -> Router {
    Router::new()
        .route("/create", post(create))
        .route("/*path", get(lookup))
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(CorsLayer::permissive())
                .layer(from_fn(logging_middleware)),
        )
        .with_state(app_state)
}

pub async fn run(host: &str, port: u16, config: Config) {
    let app_state = AppState::new(config);
    let app = create_router(app_state);

    let addr: SocketAddr = format!("{}:{}", host, port).parse().unwrap();
    info!("Server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::config::Config;
    use super::*;
    use axum_test::TestServer;
    use std::env;

    fn create_test_server(password: Option<String>) -> TestServer {
        let config = Config::new(password, env::var("DATABASE_URL").unwrap_or_else(|_| ":memory:".to_string()));
        let app_state = AppState::new(config);
        let app = create_router(app_state);
        TestServer::new(app).unwrap()
    }

    #[tokio::test]
    async fn create_post() {
        let server = create_test_server(None);
        let response = server
            .post("/create")
            .form(&[("url", "http://www.google.com/")])
            .await;

        assert_eq!(response.status_code(), 201);
    }

    #[tokio::test]
    async fn create_post_with_password() {
        let server = create_test_server(Some(String::from("secret")));
        
        let response = server
            .post("/create")
            .form(&[("url", "http://www.google.com/")])
            .await;

        assert_ne!(response.status_code(), 201);
        assert_eq!(response.status_code(), 400);

        let response = server
            .post("/create")
            .form(&[("url", "http://www.google.com/"), ("password", "secret")])
            .await;
        
        assert_eq!(response.status_code(), 201);
    }

    #[tokio::test]
    async fn create_post_with_code() {
        let server = create_test_server(None);
        let code = format!("test-{}", generate_code());
        
        let response = server
            .post("/create")
            .form(&[("url", "http://www.google.com/"), ("code", &code)])
            .await;

        assert_eq!(response.status_code(), 201);

        let location = response.header("location");
        assert_eq!(location, format!("/{}", code));
    }

    #[tokio::test]
    async fn test_lookup() {
        let subscriber = tracing_subscriber::fmt()
            .with_env_filter("hakase=info")
            .with_writer(std::io::stderr)
            .finish();

        let _ = tracing::subscriber::set_global_default(subscriber);

        let server = create_test_server(None);
        let url = "http://www.google.com";
        
        let response = server
            .post("/create")
            .form(&[("url", url)])
            .await;

        let location_header = response.header("location");
        let location = location_header.to_str().unwrap();
        
        let response = server
            .get(location)
            .await;

        assert_eq!(response.status_code(), 308); // Permanent redirect
        let redirect_header = response.header("location");
        let redirect_location = redirect_header.to_str().unwrap();
        assert_eq!(redirect_location, url);

        let response = server
            .get(&format!("{}~", location))
            .await;

        let body = response.text();
        assert_eq!(body, "1");
    }
}