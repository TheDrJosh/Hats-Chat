use std::{convert::Infallible, time::Duration};

use axum::{
    extract::{Path, State},
    response::{sse::Event, Sse},
    routing::{get, post},
    Form, Router,
};
use futures::stream::Stream;
use http::StatusCode;

use time::PrimitiveDateTime;
use tower_cookies::Cookies;

use crate::{api::auth::logged_in, data::app_state::AppState, utils::ToServerError};

pub fn chat_routes() -> Router<AppState> {
    Router::new()
        .route("/:recipient", post(post_chat))
        .route("/event/:recipient", get(sse_chat_messages))
}

#[derive(serde::Deserialize)]
struct PostChatForm {
    message: String,
}

async fn post_chat(
    Path(recipient_name): Path<String>,
    State(state): State<AppState>,
    cookies: Cookies,
    Form(form): Form<PostChatForm>,
) -> Result<StatusCode, StatusCode> {
    match logged_in(&state, &cookies).await.server_error()? {
        Some(user_id) => {
            match sqlx::query!("SELECT id FROM users WHERE username = $1", recipient_name)
                .fetch_optional(&state.pool)
                .await
                .server_error()?
                .map(|rec| rec.id)
            {
                Some(recipient_id) => {
                    let utc = time::OffsetDateTime::now_utc();
                    let timestamp = time::PrimitiveDateTime::new(utc.date(), utc.time());

                    tracing::debug!("receved message from user({user_id}) to user({recipient_id})");

                    sqlx::query!("INSERT INTO chat_messages(sender_id, recipient_id, msg, sent_at) VALUES ($1, $2, $3, $4);", user_id, recipient_id, form.message, timestamp).execute(&state.pool).await.server_error()?;

                    Ok(StatusCode::OK)
                }
                None => Err(StatusCode::BAD_REQUEST),
            }
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

async fn sse_chat_messages(
    Path(other_user_name): Path<String>,
    State(state): State<AppState>,
    cookies: Cookies,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, StatusCode> {
    tracing::debug!("sse chat start with {other_user_name}");

    let other_user_id = sqlx::query!("SELECT id FROM users WHERE username = $1", other_user_name)
        .fetch_one(&state.pool)
        .await
        .server_error()?
        .id;

    let user_id = logged_in(&state, &cookies).await.server_error()?;

    match user_id {
        Some(user_id) => {
            let mut listener = state.message_sent.subscribe();

            let stream = async_stream::stream! {
                loop {
                    listener.changed().await.unwrap();
                    let payload = *listener.borrow();

                    tracing::debug!("processing new message. payload({payload:?})");

                    if payload == (user_id, other_user_id) || payload == (other_user_id, user_id) {
                        tracing::debug!("message valid");
                        let mut msg: Vec<_> = sqlx::query!(
                            "SELECT sender_id, msg, sent_at FROM chat_messages WHERE (sender_id = $1 AND recipient_id = $2) OR (sender_id = $2 AND recipient_id = $1)",
                            user_id,
                            other_user_id
                        )
                        .fetch_all(&state.pool)
                        .await
                        .unwrap().into_iter().map(|r| (r.sender_id, r.msg, r.sent_at)).collect();

                        msg.sort_by(|(_, _, a), (_, _, b)| a.cmp(b));

                        let html = chat_messages_to_html(msg, user_id);
                        yield Ok(Event::default().event("message").data(html));
                    }
                }
            };
            Ok(Sse::new(stream).keep_alive(
                axum::response::sse::KeepAlive::new()
                    .interval(Duration::from_secs(1))
                    .text("keep-alive-text"),
            ))
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

fn chat_messages_to_html(msg: Vec<(i32, String, PrimitiveDateTime)>, this_user: i32) -> String {
    msg.into_iter()
        .map(|(sender_id, msg, time)| {
            // let is_other = sender_id != this_user;

            format!("<li>{sender_id}:  {time}\n  {msg}</li>")
        })
        .collect()
}
