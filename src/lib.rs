
#[macro_use]
extern crate log;
extern crate mime;
extern crate r2d2;
extern crate rand;
extern crate hyper;
extern crate chrono;
#[macro_use]
extern crate diesel;
extern crate gotham;
extern crate futures;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate gotham_derive;
extern crate serde_urlencoded;

use std::iter;
use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

use hyper::{Response, StatusCode, Body};
use hyper::header::Location;
use futures::{future, Stream, Future};

use gotham::state::{FromState, State};
use gotham::router::Router;
use gotham::router::builder::{build_router, DrawRoutes, DefineSingleRoute};
use gotham::handler::HandlerFuture;
use gotham::http::response::create_response;
use gotham::pipeline::new_pipeline;
use gotham::pipeline::single::single_pipeline;

mod middleware;
mod models;
mod schema;
pub mod config;

use self::middleware::{ConfigMiddleware, DieselMiddleware, ConnectionBox};
use self::config::Config;
use self::models::{Url, NewUrl};
use self::diesel::prelude::*;


#[derive(Serialize, Deserialize, Debug)]
struct CreateForm {
    url: String,
    code: Option<String>,
    password: Option<String>,
}

fn generate_code() -> String {
    let mut rng = thread_rng();
    iter::repeat(())
        .map(|()| rng.sample(Alphanumeric))
        .take(5)
        .collect()
}

impl<'a> CreateForm {
    fn ensure_code(self) -> CreateForm {
        CreateForm {
            code: self.code.or(Some(generate_code())),
            ..self
        }
    } 

    fn as_insertable(&'a self) -> NewUrl<'a> {
        NewUrl {
            myurl: self.url.as_str(),
            code: self.code.as_ref().unwrap(),
        }
    }
}

enum CreateError { Bad(&'static str) }

impl CreateError {
    fn into_response(self, state: &State) -> Response {
        match self {
            CreateError::Bad(reason) =>
                create_response(state, StatusCode::BadRequest, Some((reason.as_bytes().to_vec(), mime::TEXT_PLAIN)))
        }
        
    }
}

fn create(mut state: State) -> Box<HandlerFuture> {
    let secret = {
        let config = Box::<Config>::borrow_from(&state);

        config.secret.clone()
    };

    let body = state.take::<Body>();
    let resp = body.concat2().then(move |body| {
        // why can't we have a nice async connection pool?
        let conn = {
            let pool = state.take::<ConnectionBox>().pool;
            let pool = pool.lock().unwrap();

            pool.get().unwrap()
        };

        let result = body.map(|body| body.to_vec())
            .map_err(|_| CreateError::Bad("body parse failed."))
            .and_then(|body| {
                serde_urlencoded::from_bytes::<CreateForm>(&body[..])
                    .map_err(|_| CreateError::Bad("url decode failed"))
            })
            .and_then(|data| {
                match (data.password.clone(), secret) {
                    (Some(ref password), Some(ref secret)) if password == secret => Ok(data),
                    (_, None) => Ok(data), 
                    _ => Err(CreateError::Bad("password does not match"))
                }
            })
            .and_then(|form| {
                Ok(form.ensure_code())
            })
            .and_then(|form| {
                let result = {
                    use schema::url;
                    let insertable = form.as_insertable();
                    diesel::insert_into(url::table)
                        .values(&insertable)
                        .execute(&conn)
                        .or(Err(CreateError::Bad("can not insert into database"))) 
                };

                result.and(Ok(form))
            })
            .map(|form| {
                create_response(&state, StatusCode::Created, None)
                    .with_header(Location::new(format!("/{}", form.code.unwrap())))
            });

        let resp = match result {
            Ok(resp) => resp,
            Err(e) => e.into_response(&state)
        };

        future::ok((state, resp))
    });

    Box::new(resp)
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
struct LookupExtractor {
    #[serde(rename="*")]
    code: Vec<String>,
}

fn lookup(mut state: State) -> (State, Response) {
    let result = {
        let request_code = {
            let path = LookupExtractor::borrow_from(&state);
            path.code.join("/")
        };

        let conn = {
            let pool = state.take::<ConnectionBox>().pool;
            let pool = pool.lock().unwrap();

            pool.get().unwrap()
        };

        let result = {
            use self::schema::url::dsl::*;

            url.filter(code.eq(request_code))
                .first::<Url>(&conn)
        };

        result.map(|url| {
            url.url
        })
    };

    let resp = match result {
        Ok(url) => create_response(&state, StatusCode::MovedPermanently, None)
            .with_header(Location::new(url)),
        Err(_) => create_response(&state, StatusCode::NotFound, None)
    };

    (state, resp)
}

fn router(config: Config, thread: usize) -> Router {
    let database_url = config.database_url.clone();
    let (chain, pipelines) = single_pipeline(
        new_pipeline()
            .add(ConfigMiddleware::new(config))
            .add(DieselMiddleware::new(database_url, thread))
            .build()
    );

    build_router(chain, pipelines, |route| {
        route.post("/create").to(create);
        route.get("/*")
            .with_path_extractor::<LookupExtractor>()
            .to(lookup);
    })
}

pub fn run(host: &str, port: u16, thread: usize, config: Config) {
    gotham::start_with_num_threads((host, port), thread, router(config, thread))
}
