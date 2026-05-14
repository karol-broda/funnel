use std::sync::Arc;

use axum::extract::State;

use funnel_core::api::AccountView;

use crate::app::AppState;
use crate::auth::{Management, Scoped};
use crate::error::AppError;
use crate::response::Many;
use funnel_core::api::envelope::ErrorData;

#[utoipa::path(
    get,
    path = "/accounts",
    operation_id = "list_accounts",
    tag = "Profile",
    security(("bearer" = [])),
    responses(
        (status = 200, description = "Linked OAuth accounts", body = Vec<AccountView>),
        (status = 401, description = "Unauthorized", body = ErrorData),
    )
)]
pub async fn list(
    State(state): State<Arc<AppState>>,
    auth: Scoped<Management>,
) -> Result<Many<AccountView>, AppError> {
    let accounts = state.accounts.list_for_user(auth.user_id).await?;

    let views: Vec<AccountView> = accounts
        .into_iter()
        .map(|a| AccountView {
            id: a.id,
            provider: a.provider,
            created_at: a.created_at,
        })
        .collect();

    Ok(Many(views))
}
