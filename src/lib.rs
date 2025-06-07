use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;
use gotham::router::builder::{build_router, DefineSingleRoute, DrawRoutes};
use gotham::router::Router;

pub mod config;
mod middleware;
mod models;
mod routes;
mod schema;
mod utils;

use self::config::Config;
use self::middleware::{ConfigMiddleware, DieselMiddleware};
use self::routes::{create, lookup, LookupExtractor};
pub use self::utils::generate_code;

fn router(config: Config, thread: usize) -> Router {
    let database_url = config.database_url.clone();
    let (chain, pipelines) = single_pipeline(
        new_pipeline()
            .add(ConfigMiddleware::new(config))
            .add(DieselMiddleware::new(database_url, thread))
            .build(),
    );

    build_router(chain, pipelines, |route| {
        route.post("/create").to(create);
        route
            .get("/*")
            .with_path_extractor::<LookupExtractor>()
            .to(lookup);
    })
}

pub fn run(host: &str, port: u16, thread: usize, config: Config) {
    gotham::start_with_num_threads((host, port), thread, router(config, thread))
}

#[cfg(test)]
mod tests {
    use super::config::Config;
    use super::*;
    use gotham::test::TestServer;
    use hyper::header::Location;
    use hyper::StatusCode;
    use std::env;
    use tracing_subscriber::fmt;

    fn create_test_server(password: Option<String>) -> TestServer {
        let config = Config::new(password, env::var("DATABASE_URL").unwrap());

        TestServer::new(router(config, 1)).unwrap()
    }

    #[test]
    fn create_post() {
        let ts = create_test_server(None);
        let response = ts
            .client()
            .post(
                "http://localhost/create",
                "url=http%3A%2F%2Fwww.google.com%2F",
                mime::TEXT_PLAIN,
            )
            .perform()
            .unwrap();

        assert_eq!(response.status(), StatusCode::Created);
    }

    #[test]
    fn create_post_with_password() {
        let ts = create_test_server(Some(String::from("secret")));
        let response = ts
            .client()
            .post(
                "http://localhost/create",
                "url=http%3A%2F%2Fwww.google.com%2F",
                mime::TEXT_PLAIN,
            )
            .perform()
            .unwrap();

        assert_ne!(response.status(), StatusCode::Created);
        assert_eq!(response.status(), StatusCode::BadRequest);

        let response = ts
            .client()
            .post(
                "http://localhost/create",
                "url=http%3A%2F%2Fwww.google.com%2F&password=secret",
                mime::TEXT_PLAIN,
            )
            .perform()
            .unwrap();
        assert_eq!(response.status(), StatusCode::Created);
    }

    #[test]
    fn create_post_with_code() {
        let ts = create_test_server(None);
        let code = format!("test-{}", generate_code());
        let body = format!("url=http%3A%2F%2Fwww.google.com%2F&code={}", code);
        let response = ts
            .client()
            .post("http://localhost/create", body, mime::TEXT_PLAIN)
            .perform()
            .unwrap();

        let status = response.status();

        assert_eq!(status, StatusCode::Created);

        let location = response.headers().get::<Location>();
        let value = location.unwrap().to_string();

        assert_eq!(value, format!("/{}", code));
    }

    #[test]
    fn test_lookup() {
        let subscriber = fmt::Subscriber::builder()
            .with_env_filter("hakase=info")
            .with_writer(std::io::stderr)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set global default subscriber");

        let ts = create_test_server(None);
        let url = "http://www.google.com";
        let payload = &[("url", url)];
        let body = serde_urlencoded::to_string(payload).unwrap();

        let response = ts
            .client()
            .post("http://localhost/create", body, mime::TEXT_PLAIN)
            .perform()
            .unwrap();

        let location = response.headers().get::<Location>();
        let location = location.unwrap();

        let response = ts
            .client()
            .get(&format!("http://localhost{}", location))
            .perform()
            .unwrap();

        let result = response.headers().get::<Location>().unwrap().to_string();

        assert_eq!(result, url);

        let response = ts
            .client()
            .get(&format!("http://localhost{}~", location))
            .perform()
            .unwrap();

        let body = response.read_body().unwrap();

        assert_eq!(&body[..], b"1");
    }
}
