use axum::{
    Json,
    extract::{Form, Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Redirect},
};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::middleware::AppState;
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

#[derive(Serialize)]
pub struct ErrorResponse {
    error: String,
}

pub async fn create(
    State(app_state): State<AppState>,
    Form(form): Form<CreateForm>,
) -> Result<impl IntoResponse, impl IntoResponse> {
    let secret = &app_state.config.secret;

    // Validate password if required
    match (&form.password, secret) {
        (Some(provided_password), Some(required_secret))
            if provided_password == required_secret => {}
        (_, None) => {}
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse {
                    error: "password does not match".to_string(),
                }),
            ));
        }
    }

    let form = form.ensure_code();
    let insertable = form.as_insertable();

    let pool = app_state.pool;
    let conn = match pool.lock() {
        Ok(conn) => conn,
        Err(_) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "database connection failed".to_string(),
                }),
            ));
        }
    };

    let insert_result = conn.execute(
        "INSERT INTO url (code, url) VALUES (?1, ?2)",
        params![insertable.code, insertable.url],
    );

    match insert_result {
        Ok(_) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::LOCATION,
                format!("/{}", form.code.unwrap()).parse().unwrap(),
            );
            Ok((StatusCode::CREATED, headers))
        }
        Err(_) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "can not insert into database".to_string(),
            }),
        )),
    }
}

pub async fn lookup(
    State(app_state): State<AppState>,
    Path(code): Path<String>,
) -> axum::response::Response {
    if code.ends_with("~") {
        return lookup_count(app_state, code).await.into_response();
    }

    debug!("Looking up code: {}", code);

    let pool = app_state.pool;
    let conn = match pool.lock() {
        Ok(conn) => conn,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let result = {
        let mut stmt = match conn
            .prepare("SELECT id, code, url, create_time, count FROM url WHERE code = ?1")
        {
            Ok(stmt) => stmt,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };

        let url_result = stmt.query_row(params![code], |row| Url::from_row(row));

        url_result
            .and_then(|url| {
                // Update the count
                conn.execute(
                    "UPDATE url SET count = count + 1 WHERE id = ?1",
                    params![url.id],
                )?;
                Ok(url)
            })
            .map(|url| url.url)
    };

    match result {
        Ok(url) => Redirect::permanent(&url).into_response(),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn lookup_count(app_state: AppState, mut request_code: String) -> axum::response::Response {
    let trimmed = request_code.len() - 1;
    request_code.truncate(trimmed);

    debug!("Looking up count: {}", request_code);

    let pool = app_state.pool;
    let conn = match pool.lock() {
        Ok(conn) => conn,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    let result = {
        let mut stmt = match conn
            .prepare("SELECT id, code, url, create_time, count FROM url WHERE code = ?1")
        {
            Ok(stmt) => stmt,
            Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };

        stmt.query_row(params![request_code], |row| Url::from_row(row))
            .map(|url| url.count)
    };

    match result {
        Ok(count) => {
            let mut headers = HeaderMap::new();
            headers.insert(header::CONTENT_TYPE, "text/plain".parse().unwrap());
            (StatusCode::OK, headers, format!("{}", count)).into_response()
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}
