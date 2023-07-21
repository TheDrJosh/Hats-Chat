use http::StatusCode;
use std::fmt::Debug;

pub mod username;
pub mod auth_layer;

pub trait ToServerError<T, E> {
    fn server_error(self) -> Result<T, (StatusCode, String)>;
}

impl<T, E> ToServerError<T, E> for Result<T, E> 
where
    E: Debug
{
    fn server_error(self) -> Result<T, (StatusCode, String)> {
        self.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("{e:?}")))
    }
}

