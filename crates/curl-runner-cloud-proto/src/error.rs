use serde::{Deserialize, Serialize};

/// Machine-readable error codes returned in `ErrorBody`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidRequest,
    InvalidBundle,
    Unauthorized,
    Forbidden,
    NotFound,
    PlanLimit,
    WrongSource,
    Conflict,
    RateLimited,
    CodeExpired,
    CodeInvalid,
    SignupClosed,
    /// The account is suspended: no sign-in by any method.
    AccountSuspended,
    /// The team is suspended: publishing to it is refused.
    TeamSuspended,
    Unavailable,
    /// A code from a newer server that this build does not know.
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_code_wire_names() {
        assert_eq!(
            serde_json::to_string(&ErrorCode::AccountSuspended).unwrap(),
            "\"account_suspended\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::TeamSuspended).unwrap(),
            "\"team_suspended\""
        );
        assert_eq!(
            serde_json::to_string(&ErrorCode::WrongSource).unwrap(),
            "\"wrong_source\""
        );
        assert_eq!(
            serde_json::from_str::<ErrorCode>("\"wrong_source\"").unwrap(),
            ErrorCode::WrongSource
        );
        assert_eq!(
            serde_json::from_str::<ErrorCode>("\"from_the_future\"").unwrap(),
            ErrorCode::Unknown
        );
    }
}
