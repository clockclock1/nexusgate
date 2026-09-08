use axum::body::Body;
use axum::http::{header, HeaderValue, Request, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "../web/dist/"]
struct Assets;

fn normalize_path(uri: &Uri) -> String {
    let path = uri.path().trim_start_matches('/');
    if path.is_empty() {
        "index.html".into()
    } else {
        path.to_string()
    }
}

fn asset_response(path: &str) -> Option<Response> {
    let file = Assets::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let mut res = Response::new(Body::from(file.data.into_owned()));
    if let Ok(val) = HeaderValue::from_str(mime.essence_str()) {
        res.headers_mut().insert(header::CONTENT_TYPE, val);
    }
    Some(res)
}

/// Serve embedded SPA assets; unknown routes fall back to index.html for client routing.
pub async fn serve_spa(req: Request<Body>) -> Response {
    let path = normalize_path(req.uri());

    if let Some(res) = asset_response(&path) {
        return res;
    }

    // Try with trailing index for directories
    let as_index = format!("{}/index.html", path.trim_end_matches('/'));
    if let Some(res) = asset_response(&as_index) {
        return res;
    }

    // SPA fallback
    if let Some(res) = asset_response("index.html") {
        return res;
    }

    (StatusCode::NOT_FOUND, "frontend not embedded; build web/dist first").into_response()
}

pub fn embedded_asset_count() -> usize {
    Assets::iter().count()
}
