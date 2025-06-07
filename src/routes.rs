use futures::{future, Future, Stream};
use hyper::header::Location;
use hyper::{Body, Response, StatusCode};

use diesel::prelude::*;
use gotham::handler::HandlerFuture;
use gotham::http::response::create_response;
use gotham::state::{FromState, State};
use gotham_derive::StateData;
use gotham_derive::StaticResponseExtender;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use tracing::debug;

use crate::config::Config;
use crate::middleware::ConnectionBox;
use crate::models::{NewUrl, Url};
use crate::schema;

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateForm {
    pub url: String,
    pub code: Option<String>,
    pub password: Option<String>,
}

impl CreateForm {
    pub fn ensure_code(self) -> CreateForm {
        CreateForm {
            code: self.code.or(Some(crate::generate_code())),
            ..self
        }
    }

    pub fn as_insertable(&self) -> NewUrl {
        NewUrl {
            myurl: self.url.as_str(),
            code: self.code.as_ref().unwrap(),
        }
    }
}

pub enum CreateError {
    Bad(&'static str),
}

impl CreateError {
    pub fn into_response(self, state: &State) -> Response {
        match self {
            CreateError::Bad(reason) => create_response(
                state,
                StatusCode::BadRequest,
                Some((reason.as_bytes().to_vec(), mime::TEXT_PLAIN)),
            ),
        }
    }
}

pub fn create(mut state: State) -> Box<HandlerFuture> {
    let secret = {
        let config = Box::<Config>::borrow_from(&state);
        config.secret.clone()
    };

    let body = state.take::<Body>();
    let resp = body.concat2().then(move |body| {
        // why can't we have a nice async connection pool?
        let mut conn = {
            let pool = state.take::<ConnectionBox>().pool;
            let pool = pool.lock().unwrap();
            pool.get().unwrap()
        };

        let result = body
            .map(|body| body.to_vec())
            .map_err(|_| CreateError::Bad("body parse failed."))
            .and_then(|body| {
                serde_urlencoded::from_bytes::<CreateForm>(&body[..])
                    .map_err(|_| CreateError::Bad("url decode failed"))
            })
            .and_then(|data| match (data.password.clone(), secret) {
                (Some(ref password), Some(ref secret)) if password == secret => Ok(data),
                (_, None) => Ok(data),
                _ => Err(CreateError::Bad("password does not match")),
            })
            .and_then(|form| Ok(form.ensure_code()))
            .and_then(|form| {
                let result = {
                    use schema::url;
                    let insertable = form.as_insertable();
                    diesel::insert_into(url::table)
                        .values(&insertable)
                        .execute(&mut conn)
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
            Err(e) => e.into_response(&state),
        };

        future::ok((state, resp))
    });

    Box::new(resp)
}

#[derive(Deserialize, StateData, StaticResponseExtender)]
pub struct LookupExtractor {
    #[serde(rename = "*")]
    pub code: Vec<String>,
}

pub fn lookup(mut state: State) -> (State, Response) {
    let request_code = {
        let path = LookupExtractor::borrow_from(&state);
        path.code.join("/")
    };

    if request_code.ends_with("~") {
        return lookup_count(state, request_code);
    }

    debug!("Looking up code: {}", request_code);

    let mut conn = {
        let pool = state.take::<ConnectionBox>().pool;
        let pool = pool.lock().unwrap();
        pool.get().unwrap()
    };

    let result = {
        let result = {
            use crate::schema::url::dsl::*;
            url.filter(code.eq(request_code)).first::<Url>(&mut conn)
        };

        result
            .and_then(|result| {
                use crate::schema::url::dsl::{count, url};
                let _ = diesel::update(url.find(result.id))
                    .set(count.eq(count + 1))
                    .execute(&mut conn);

                Ok(result)
            })
            .map(|url| url.myurl)
    };

    let resp = match result {
        Ok(url) => create_response(&state, StatusCode::MovedPermanently, None)
            .with_header(Location::new(url)),
        Err(_) => create_response(&state, StatusCode::NotFound, None),
    };

    (state, resp)
}

pub fn lookup_count(mut state: State, mut request_code: String) -> (State, Response) {
    let result = {
        let trimmed = request_code.len() - 1;
        request_code.truncate(trimmed);

        debug!("Looking up count: {}", request_code);

        let mut conn = {
            let pool = state.take::<ConnectionBox>().pool;
            let pool = pool.lock().unwrap();
            pool.get().unwrap()
        };

        let result = {
            use crate::schema::url::dsl::*;
            url.filter(code.eq(request_code)).first::<Url>(&mut conn)
        };

        result.map(|url| url.count)
    };

    let resp = match result {
        Ok(count) => create_response(
            &state,
            StatusCode::Ok,
            Some((format!("{}", count).into_bytes().to_vec(), mime::TEXT_PLAIN)),
        ),
        Err(_) => create_response(&state, StatusCode::NotFound, None),
    };

    (state, resp)
}
