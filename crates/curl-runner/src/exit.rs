//! Process exit code (TS `determineExitCode`, cli.ts:554-580).

use curl_runner_core::model::{ExecutionSummary, GlobalConfig};

/// Exit code after a normal run.
pub fn determine(summary: &ExecutionSummary, global: &GlobalConfig) -> i32 {
    let (failed, total) = (summary.failed, summary.total);
    if failed == 0 {
        return 0;
    }
    if let Some(ci) = &global.ci {
        if ci.strict_exit == Some(true) {
            return 1;
        }
        if ci.fail_on.is_some_and(|n| failed > n) {
            return 1;
        }
        if let Some(pct) = ci.fail_on_percentage
            && total > 0
            && (failed as f64 / total as f64) * 100.0 > pct
        {
            return 1;
        }
        if ci.fail_on.is_some() || ci.fail_on_percentage.is_some() {
            return 0;
        }
    }
    i32::from(global.continue_on_error != Some(true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use curl_runner_core::model::CiExitConfig;

    fn summary(total: u64, failed: u64) -> ExecutionSummary {
        ExecutionSummary {
            total,
            successful: total - failed,
            failed,
            skipped: 0,
            duration: 1000.0,
            results: vec![],
        }
    }

    fn cfg(coe: Option<bool>, ci: Option<CiExitConfig>) -> GlobalConfig {
        GlobalConfig {
            continue_on_error: coe,
            ci,
            ..Default::default()
        }
    }

    fn ci(strict: Option<bool>, fail_on: Option<u64>, pct: Option<f64>) -> Option<CiExitConfig> {
        Some(CiExitConfig {
            strict_exit: strict,
            fail_on,
            fail_on_percentage: pct,
        })
    }

    // ci-exit.test.ts
    #[test]
    fn default_behavior() {
        assert_eq!(determine(&summary(10, 0), &cfg(None, None)), 0);
        assert_eq!(determine(&summary(10, 2), &cfg(None, None)), 1);
        assert_eq!(determine(&summary(10, 2), &cfg(Some(true), None)), 0);
    }

    #[test]
    fn strict_exit() {
        assert_eq!(
            determine(&summary(10, 1), &cfg(None, ci(Some(true), None, None))),
            1
        );
        assert_eq!(
            determine(&summary(10, 0), &cfg(None, ci(Some(true), None, None))),
            0
        );
        assert_eq!(
            determine(
                &summary(10, 1),
                &cfg(Some(true), ci(Some(true), None, None))
            ),
            1
        );
    }

    #[test]
    fn fail_on_threshold() {
        assert_eq!(
            determine(&summary(10, 2), &cfg(None, ci(None, Some(2), None))),
            0
        );
        assert_eq!(
            determine(&summary(10, 3), &cfg(None, ci(None, Some(2), None))),
            1
        );
        assert_eq!(
            determine(&summary(10, 0), &cfg(None, ci(None, Some(0), None))),
            0
        );
        assert_eq!(
            determine(&summary(10, 1), &cfg(None, ci(None, Some(0), None))),
            1
        );
    }

    #[test]
    fn fail_on_percentage() {
        assert_eq!(
            determine(&summary(100, 10), &cfg(None, ci(None, None, Some(10.0)))),
            0
        );
        assert_eq!(
            determine(&summary(100, 11), &cfg(None, ci(None, None, Some(10.0)))),
            1
        );
        assert_eq!(
            determine(&summary(10, 5), &cfg(None, ci(None, None, Some(50.0)))),
            0
        );
        assert_eq!(
            determine(&summary(100, 51), &cfg(None, ci(None, None, Some(50.0)))),
            1
        );
        assert_eq!(
            determine(&summary(0, 0), &cfg(None, ci(None, None, Some(10.0)))),
            0
        );
    }

    #[test]
    fn combined_options() {
        assert_eq!(
            determine(&summary(10, 1), &cfg(None, ci(Some(true), Some(5), None))),
            1
        );
        assert_eq!(
            determine(
                &summary(100, 1),
                &cfg(None, ci(Some(true), None, Some(50.0)))
            ),
            1
        );
        assert_eq!(
            determine(&summary(100, 6), &cfg(None, ci(None, Some(5), Some(10.0)))),
            1
        );
        assert_eq!(
            determine(&summary(100, 5), &cfg(None, ci(None, Some(5), Some(10.0)))),
            0
        );
    }

    #[test]
    fn ci_with_continue_on_error() {
        assert_eq!(
            determine(&summary(10, 3), &cfg(Some(true), ci(None, Some(2), None))),
            1
        );
        assert_eq!(
            determine(&summary(10, 2), &cfg(Some(true), ci(None, Some(2), None))),
            0
        );
    }

    #[test]
    fn empty_ci_block_falls_back_to_continue_on_error() {
        let empty = ci(None, None, None);
        assert_eq!(determine(&summary(10, 1), &cfg(None, empty.clone())), 1);
        assert_eq!(determine(&summary(10, 1), &cfg(Some(true), empty)), 0);
        // strictExit: false is not strict
        assert_eq!(
            determine(
                &summary(10, 1),
                &cfg(Some(true), ci(Some(false), None, None))
            ),
            0
        );
    }
}
