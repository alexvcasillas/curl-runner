//! What each plan allows. The team's effective plan
//! ([`crate::billing::effective::effective_plan`]) and the server settings are
//! the only inputs.

use curl_runner_cloud_proto::Plan;

use crate::settings::Settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Accepted members plus pending invites; `None` is unlimited.
    pub members: Option<u32>,
    /// Whether restricted (invite-only) collections are allowed.
    pub restricted: bool,
}

/// The Free limits come from `settings`; Team has none.
pub fn limits(plan: Plan, settings: &Settings) -> Limits {
    match plan {
        Plan::Free => Limits {
            members: Some(settings.free_member_limit),
            restricted: settings.free_restricted,
        },
        Plan::Team => Limits {
            members: None,
            restricted: true,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn free_defaults_to_three_members_and_unrestricted_only() {
        assert_eq!(
            limits(Plan::Free, &Settings::default()),
            Limits {
                members: Some(3),
                restricted: false
            }
        );
    }

    #[test]
    fn free_follows_the_settings() {
        let settings = Settings {
            free_member_limit: 7,
            free_restricted: true,
            ..Settings::default()
        };
        assert_eq!(
            limits(Plan::Free, &settings),
            Limits {
                members: Some(7),
                restricted: true
            }
        );
    }

    #[test]
    fn team_has_no_member_cap_and_allows_restricted() {
        let settings = Settings {
            free_member_limit: 1,
            ..Settings::default()
        };
        assert_eq!(
            limits(Plan::Team, &settings),
            Limits {
                members: None,
                restricted: true
            }
        );
    }
}
