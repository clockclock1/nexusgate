use crate::state::AppState;
use axum::extract::{Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use p2p_security::verify_password;
use serde::{Deserialize, Serialize};
use sqlx::Row;

#[derive(Deserialize)]
pub struct LoginReq {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResp {
    pub token: String,
    pub user: UserDto,
    pub expires_at: Option<String>,
}

#[derive(Serialize, Clone)]
pub struct UserDto {
    pub user_id: String,
    pub username: String,
    pub role: String,
    pub enabled: bool,
    pub email: Option<String>,
    pub last_login: Option<String>,
    pub created_at: Option<String>,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginReq>,
) -> Result<Json<LoginResp>, AppError> {
    let row = sqlx::query("SELECT * FROM users WHERE username = ?")
        .bind(&req.username)
        .fetch_optional(&state.db)
        .await
        .map_err(AppError::internal)?;
    let Some(row) = row else {
        return Err(AppError::unauthorized("invalid credentials"));
    };
    let hash: String = row.get("password_hash");
    let enabled: i64 = row.get("enabled");
    if enabled != 1 || !verify_password(&req.password, &hash) {
        return Err(AppError::unauthorized("invalid credentials"));
    }
    let user_id: String = row.get("id");
    let role: String = row.get("role");
    let token = state
        .jwt
        .issue(&user_id, &role)
        .map_err(AppError::internal)?;
    let _ = sqlx::query("UPDATE users SET last_login = ? WHERE id = ?")
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(&user_id)
        .execute(&state.db)
        .await;
    let _ = crate::db::audit(&state.db, &req.username, "login", "ok").await;
    Ok(Json(LoginResp {
        token,
        user: UserDto {
            user_id,
            username: row.get("username"),
            role,
            enabled: true,
            email: row.try_get("email").ok(),
            last_login: Some(chrono::Utc::now().to_rfc3339()),
            created_at: row.try_get("created_at").ok(),
        },
        expires_at: None,
    }))
}

pub async fn logout() -> impl IntoResponse {
    StatusCode::OK
}

pub async fn me(State(state): State<AppState>, req: Request) -> Result<Json<UserDto>, AppError> {
    let user_id = req
        .extensions()
        .get::<AuthUser>()
        .map(|u| u.user_id.clone())
        .ok_or_else(|| AppError::unauthorized("missing auth"))?;
    let row = sqlx::query("SELECT * FROM users WHERE id = ?")
        .bind(&user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::unauthorized("user not found"))?;
    Ok(Json(UserDto {
        user_id: row.get("id"),
        username: row.get("username"),
        role: row.get("role"),
        enabled: row.get::<i64, _>("enabled") == 1,
        email: row.try_get("email").ok(),
        last_login: row.try_get("last_login").ok(),
        created_at: row.try_get("created_at").ok(),
    }))
}

#[derive(Clone)]
pub struct AuthUser {
    pub user_id: String,
    #[allow(dead_code)]
    pub role: String,
}

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Allow websocket upgrade paths to pass token via query
    let path = req.uri().path().to_string();
    let token = extract_token(&req);
    let Some(token) = token else {
        if path.starts_with("/ws/") {
            // try query ?token=
            if let Some(q) = req.uri().query() {
                for part in q.split('&') {
                    if let Some(t) = part.strip_prefix("token=") {
                        let claims = state.jwt.verify(t).map_err(|_| AppError::unauthorized("bad token"))?;
                        req.extensions_mut().insert(AuthUser {
                            user_id: claims.sub,
                            role: claims.role,
                        });
                        return Ok(next.run(req).await);
                    }
                }
            }
        }
        return Err(AppError::unauthorized("missing bearer token"));
    };
    let claims = state
        .jwt
        .verify(&token)
        .map_err(|_| AppError::unauthorized("invalid token"))?;
    req.extensions_mut().insert(AuthUser {
        user_id: claims.sub,
        role: claims.role,
    });
    Ok(next.run(req).await)
}

fn extract_token(req: &Request) -> Option<String> {
    req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    message: String,
}

impl AppError {
    pub fn unauthorized(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: msg.into(),
        }
    }
    #[allow(dead_code)]
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: msg.into(),
        }
    }
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: msg.into(),
        }
    }
    pub fn internal(err: impl std::fmt::Display) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: err.to_string(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(serde_json::json!({ "message": self.message, "code": self.status.as_u16() })),
        )
            .into_response()
    }
}

impl From<p2p_common::Error> for AppError {
    fn from(e: p2p_common::Error) -> Self {
        Self::internal(e)
    }
}
