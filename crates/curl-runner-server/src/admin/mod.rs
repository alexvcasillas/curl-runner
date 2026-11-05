//! The admin panel's API, all under `/v1/admin`. Every route takes
//! [`AdminAuth`] first, so anyone who is not an admin gets the same `404` as
//! an unknown path, whatever they send. Every write records one
//! [`events::Event`] in its own transaction.

pub mod activity;
pub mod admins;
pub mod auth;
pub mod csv;
pub mod events;

use axum::Router;
use axum::routing::{get, patch, put};

use crate::error::ApiError;
use crate::state::AppState;

pub use auth::{AdminAuth, Perm, Role};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/v1/admin/me", get(admins::me))
        .route("/v1/admin/admins", get(admins::list).post(admins::add))
        .route(
            "/v1/admin/admins/{user_id}",
            patch(admins::update).delete(admins::remove),
        )
        .route("/v1/admin/activity", get(activity::list))
        .route("/v1/admin/activity.csv", get(activity::export))
        .route("/v1/admin/teams/{team}/plan", put(crate::teams::set_plan))
        // A wrong method is a 404 too: a 405 would confirm the path exists.
        .method_not_allowed_fallback(not_found)
}

async fn not_found() -> ApiError {
    ApiError::not_found()
}
