use std::{collections::HashMap, convert::Infallible, time::Duration};

use askama::Template;
use axum::{
    extract::{Path, State},
    response::{sse::Event, Sse},
    routing::{get, post},
    Form, Router,
};
use futures::{stream::Stream, StreamExt};
use http::StatusCode;

use sqlx::PgPool;
use time::PrimitiveDateTime;
use tower_cookies::Cookies;

use crate::{api::auth::logged_in, app::BaseInfo, data::app_state::AppState, utils::{ToServerError, username::Username}};

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

                    state
                        .message_sent
                        .send((user_id, recipient_id))
                        .server_error()?;

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

                        let chat_window = ChatWindow {
                            base_info: BaseInfo::new(user_id, &state.pool).await.unwrap(),
                            chat_window_info: Some(ChatWindowInfo::new(user_id, other_user_id, &state.pool).await.unwrap()),
                        };

                        let html = chat_window.render().unwrap().replace(&['\n', '\r'], "");

                        tracing::debug!("SSE responce sent to user({user_id})");

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

#[derive(Template)]
#[template(path = "components/chat_window.html")]
pub struct ChatWindow {
    pub base_info: BaseInfo,
    pub chat_window_info: Option<ChatWindowInfo>,
}

pub struct ChatWindowInfo {
    pub messages: Vec<(i32, String, PrimitiveDateTime)>,
    pub usernames: HashMap<i32, Username>,
    pub recipient_name: String,
}

impl ChatWindowInfo {
    pub async fn new(user_id: i32, other_user_id: i32, pool: &PgPool) -> anyhow::Result<Self> {
        tracing::debug!("retriving messages between user({user_id}) and user({other_user_id})");

        let recipient_name =
            sqlx::query!("SELECT username FROM users WHERE id = $1", other_user_id)
                .fetch_one(pool)
                .await?
                .username;

        let usernames = sqlx::query!(
            "SELECT id, display_name, username FROM users WHERE id = $1 OR id = $2",
            user_id,
            other_user_id
        )
        .fetch(pool)
        .map(|rec| {
            let rec = rec.unwrap();
            (rec.id, Username::new(rec.username, rec.display_name))
        })
        .collect::<HashMap<_, _>>()
        .await;
        let mut messages: Vec<_> = sqlx::query!(
            "SELECT sender_id, msg, sent_at FROM chat_messages WHERE (sender_id = $1 AND recipient_id = $2) OR (sender_id = $2 AND recipient_id = $1)",
            user_id,
            other_user_id
        )
        .fetch_all(pool)
        .await
        .unwrap().into_iter().map(|r| (r.sender_id, r.msg, r.sent_at)).collect();

        messages.sort_by(|(_, _, a), (_, _, b)| a.cmp(b));

        Ok(Self {
            messages,
            usernames,
            recipient_name,
        })
    }
}

