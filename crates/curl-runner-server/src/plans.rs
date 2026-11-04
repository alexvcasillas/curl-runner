//! What each plan allows. The team's `plan` column is the only input.

use curl_runner_cloud_proto::Plan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// Accepted members plus pending invites; `None` is unlimited.
    pub members: Option<u32>,
    /// Whether restricted (invite-only) collections are allowed.
    pub restricted: bool,
}

pub fn limits(plan: Plan) -> Limits {
    match plan {
        Plan::Free => Limits {
            members: Some(3),
            restricted: false,
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
    fn free_is_three_members_and_unrestricted_only() {
        assert_eq!(
            limits(Plan::Free),
            Limits {
                members: Some(3),
                restricted: false
            }
        );
    }

    #[test]
    fn team_has_no_member_cap_and_allows_restricted() {
        assert_eq!(
            limits(Plan::Team),
            Limits {
                members: None,
                restricted: true
            }
        );
    }
}
