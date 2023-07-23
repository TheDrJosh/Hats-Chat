use std::io::Cursor;

use axum::extract::{Multipart, State};
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};

use crate::{
    data::app_state::AppState,
    utils::{auth_layer::ExtractActivatedAuth, ToServerError},
};

pub async fn change_display_name(
    State(state): State<AppState>,
    ExtractActivatedAuth(user_id): ExtractActivatedAuth,
    mut multipart: Multipart,
) -> Result<HeaderMap, (StatusCode, String)> {
    tracing::debug!("starting update to profile picture for user({user_id}");

    let (file_type, file) = match multipart.next_field().await.server_error()? {
        Some(field) => {
            let name = field
                .name()
                .ok_or((StatusCode::BAD_REQUEST, String::from("Bad Request")))?
                .to_owned();
            let content_type = field
                .content_type()
                .ok_or((StatusCode::BAD_REQUEST, String::from("Bad Request")))?
                .to_owned();
            let data = field.bytes().await.server_error()?;

            if name != "file" {
                tracing::debug!("unexpected parameter name got ({name})");
                Err((StatusCode::BAD_REQUEST, String::from("Bad Request")))?;
            }

            if !content_type.starts_with("image/") {
                tracing::debug!("unexpected content type got ({content_type})");
                Err((StatusCode::BAD_REQUEST, String::from("Bad Request")))?;
            }

            if multipart.next_field().await.server_error()?.is_some() {
                tracing::debug!("unexpected got second part");
                Err((StatusCode::BAD_REQUEST, String::from("Bad Request")))?;
            }

            (content_type, data)
        }
        None => Err((StatusCode::BAD_REQUEST, String::from("Bad Request")))?,
    };

    tracing::debug!("decoded multipart form for new profile picture");

    let file_type = image::ImageFormat::from_mime_type(file_type)
        .ok_or((StatusCode::BAD_REQUEST, String::from("Bad Request")))?;

    let img = image::load_from_memory_with_format(&file, file_type).server_error()?;

    let mut avif_img = Vec::new();

    img.write_to(
        &mut Cursor::new(&mut avif_img),
        image::ImageOutputFormat::Avif,
    )
    .server_error()?;

    sqlx::query!(
        "UPDATE users SET profile_picture = $1 WHERE id = $2",
        avif_img,
        user_id
    )
    .execute(&state.pool)
    .await
    .server_error()?;

    let mut headers = HeaderMap::default();

    headers.insert(
        HeaderName::from_static("hx-refresh"),
        HeaderValue::from_static("true"),
    );

    Ok(headers)
}
