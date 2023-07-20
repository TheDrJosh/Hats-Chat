use axum::{routing::put, Router};

use crate::data::app_state::AppState;

mod change_display_name;
mod chage_profile_picture;

pub fn account_details_uris() -> Router<AppState> {
    Router::new().route(
        "/display_name",
        put(change_display_name::change_display_name),
    ).route("/profile_picture", put(chage_profile_picture::change_display_name))
}
