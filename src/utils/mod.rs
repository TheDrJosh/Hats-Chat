use http::StatusCode;

pub mod username;

pub trait ToServerError<T, E> {
    fn server_error(self) -> Result<T, StatusCode>;
}

impl<T, E> ToServerError<T, E> for Result<T, E> {
    fn server_error(self) -> Result<T, StatusCode> {
        self.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
}
