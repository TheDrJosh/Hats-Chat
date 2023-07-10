use http::StatusCode;

pub trait ToServerError<T, E> {
    fn server_error(self) -> Result<T, StatusCode>;
}

impl<T, E> ToServerError<T, E> for Result<T, E> {
    fn server_error(self) -> Result<T, StatusCode> {
        self.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
}

pub trait RowOptional<T> {
    fn optional(self) -> sqlx::Result<Option<T>>;
}

impl<T> RowOptional<T> for sqlx::Result<T> {
    fn optional(self) -> sqlx::Result<Option<T>> {
        match self {
            Ok(rec) => Ok(Some(rec)),
            Err(sqlx::Error::RowNotFound) => Ok(None),
            Err(e) => Err(e),
        }
    }
}
