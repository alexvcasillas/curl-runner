//! Who may use the admin panel, and the `AdminAuth` extractor.

use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use curl_runner_cloud_proto::ErrorCode;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::UserAuth;
use crate::error::ApiError;
use crate::extract::ClientIp;
use crate::state::AppState;
use crate::teams::admin_role;

/// An owner comes from `OPERATOR_EMAILS` (or is the recorded first operator)
/// and has every permission. An admin is a row in `admins`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Owner,
    Admin,
}

/// What an admin may change. Every admin may read everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Perm {
    Users,
    Teams,
    Billing,
    Emails,
    Settings,
    Admins,
}

impl Perm {
    pub const ALL: [Perm; 6] = [
        Perm::Users,
        Perm::Teams,
        Perm::Billing,
        Perm::Emails,
        Perm::Settings,
        Perm::Admins,
    ];

    /// The string stored in `admins.permissions`.
    pub fn as_str(self) -> &'static str {
        match self {
            Perm::Users => "users",
            Perm::Teams => "teams",
            Perm::Billing => "billing",
            Perm::Emails => "emails",
            Perm::Settings => "settings",
            Perm::Admins => "admins",
        }
    }

    pub fn parse(s: &str) -> Option<Perm> {
        Perm::ALL.into_iter().find(|p| p.as_str() == s)
    }

    /// Deduplicated, in [`Perm::ALL`] order.
    pub fn normalize(perms: &[Perm]) -> Vec<Perm> {
        Perm::ALL
            .into_iter()
            .filter(|p| perms.contains(p))
            .collect()
    }

    /// Stored strings as permissions. Unknown strings are dropped.
    pub fn from_column(values: &[String]) -> Vec<Perm> {
        let parsed: Vec<Perm> = values.iter().filter_map(|v| Perm::parse(v)).collect();
        Perm::normalize(&parsed)
    }

    pub fn to_column(perms: &[Perm]) -> Vec<String> {
        perms.iter().map(|p| p.as_str().to_owned()).collect()
    }
}

/// A signed-in owner or admin. Anyone else, signed in or not, gets the same
/// `404 not_found` as an unknown path, so the panel's existence does not leak.
/// Put it first in a handler's arguments: an extractor that runs before it
/// could answer a non-admin with a 400.
#[derive(Debug, Clone)]
pub struct AdminAuth {
    pub user_id: Uuid,
    pub email: String,
    pub role: Role,
    pub permissions: Vec<Perm>,
    /// The caller's IP, recorded on each admin event.
    pub ip: String,
}

impl AdminAuth {
    /// `403 forbidden` unless the admin has `perm`.
    pub fn require(&self, perm: Perm) -> Result<(), ApiError> {
        if self.permissions.contains(&perm) {
            Ok(())
        } else {
            Err(ApiError::new(
                ErrorCode::Forbidden,
                "You don't have permission to do that.",
            ))
        }
    }
}

impl FromRequestParts<AppState> for AdminAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = match UserAuth::from_request_parts(parts, state).await {
            Ok(user) => user,
            Err(err) if err.code == ErrorCode::Unauthorized => return Err(ApiError::not_found()),
            Err(err) => return Err(err),
        };
        let (role, permissions) = admin_role(&state.db, &state.config, user.user_id, &user.email)
            .await?
            .ok_or_else(ApiError::not_found)?;
        let Ok(ClientIp(ip)) = ClientIp::from_request_parts(parts, state).await;
        Ok(Self {
            user_id: user.user_id,
            email: user.email,
            role,
            permissions,
            ip,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perms_round_trip_and_normalize() {
        for p in Perm::ALL {
            assert_eq!(Perm::parse(p.as_str()), Some(p));
            assert_eq!(
                serde_json::to_value(p).unwrap(),
                serde_json::Value::String(p.as_str().into())
            );
        }
        assert_eq!(
            Perm::from_column(&[
                "admins".into(),
                "root".into(),
                "users".into(),
                "users".into()
            ]),
            [Perm::Users, Perm::Admins]
        );
    }

    #[test]
    fn require_checks_the_list() {
        let admin = AdminAuth {
            user_id: Uuid::nil(),
            email: "rosa@example.com".into(),
            role: Role::Admin,
            permissions: vec![Perm::Users],
            ip: "unknown".into(),
        };
        assert!(admin.require(Perm::Users).is_ok());
        let err = admin.require(Perm::Billing).unwrap_err();
        assert_eq!(err.code, ErrorCode::Forbidden);
        assert_eq!(err.message, "You don't have permission to do that.");
    }
}
