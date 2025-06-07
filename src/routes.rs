use futures::{future, Future, Stream};
use hyper::header::Location;
use hyper::{Body, Response, StatusCode};

use rusqlite::params;
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
        NewUrl::new(self.url.as_str(), self.code.as_ref().unwrap())
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
        let pool = state.take::<ConnectionBox>().pool;
        let conn = pool.lock().unwrap();

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
                let insertable = form.as_insertable();
                let insert_result = conn.execute(
                    "INSERT INTO url (code, url) VALUES (?1, ?2)",
                    params![insertable.code, insertable.myurl],
                );

                match insert_result {
                    Ok(_) => Ok(form),
                    Err(_) => Err(CreateError::Bad("can not insert into database")),
                }
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

    let pool = state.take::<ConnectionBox>().pool;
    let conn = pool.lock().unwrap();

    let result = {
        let mut stmt = conn.prepare("SELECT id, code, url, create_time, count FROM url WHERE code = ?1").unwrap();
        let url_result = stmt.query_row(params![request_code], |row| {
            Url::from_row(row)
        });

        url_result
            .and_then(|url| {
                // Update the count
                conn.execute(
                    "UPDATE url SET count = count + 1 WHERE id = ?1",
                    params![url.id],
                )?;
                Ok(url)
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

        let pool = state.take::<ConnectionBox>().pool;
        let conn = pool.lock().unwrap();

        let mut stmt = conn.prepare("SELECT id, code, url, create_time, count FROM url WHERE code = ?1").unwrap();
        let result = stmt.query_row(params![request_code], |row| {
            Url::from_row(row)
        });

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