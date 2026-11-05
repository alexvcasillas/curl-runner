//! The admin panel's API, all under `/v1/admin`. Every route takes
//! [`AdminAuth`] first, so anyone who is not an admin gets the same `404` as
//! an unknown path, whatever they send. Every write records one
//! [`events::Event`] in its own transaction.

pub mod activity;
pub mod admins;
pub mod auth;
pub mod csv;
pub mod events;
pub mod settings;
pub mod users;

use axum::Router;
use axum::routing::{delete, get, patch, post, put};

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
        .route("/v1/admin/settings", get(settings::get))
        .route("/v1/admin/settings/test-email", post(settings::test_email))
        .route("/v1/admin/settings/{section}", put(settings::update))
        .route("/v1/admin/users", get(users::list))
        .route("/v1/admin/users.csv", get(users::export))
        .route("/v1/admin/users/bulk", post(users::bulk))
        .route(
            "/v1/admin/users/{id}",
            get(users::detail).delete(users::delete),
        )
        .route("/v1/admin/users/{id}/suspend", post(users::suspend))
        .route("/v1/admin/users/{id}/unsuspend", post(users::unsuspend))
        .route(
            "/v1/admin/users/{id}/sign-out-everywhere",
            post(users::sign_out_everywhere),
        )
        .route(
            "/v1/admin/users/{id}/sessions/{sid}",
            delete(users::revoke_session),
        )
        .route(
            "/v1/admin/users/{id}/identities/{provider}",
            delete(users::unlink_identity),
        )
        .route("/v1/admin/users/{id}/email", post(users::email))
        .route("/v1/admin/teams/{team}/plan", put(crate::teams::set_plan))
        // A wrong method is a 404 too: a 405 would confirm the path exists.
        .method_not_allowed_fallback(not_found)
}

async fn not_found() -> ApiError {
    ApiError::not_found()
}
