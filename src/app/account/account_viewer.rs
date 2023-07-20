use askama::Template;
use http::StatusCode;

pub async fn account_viewer_page() -> Result<AccountViewerTemplate, StatusCode> {
    Ok(AccountViewerTemplate)
}

#[derive(Template, Default)]
#[template(path = "account_viewer.html")]
pub struct AccountViewerTemplate ;
