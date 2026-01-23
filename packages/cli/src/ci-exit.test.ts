import { describe, expect, test } from 'bun:test'
import type { ExecutionSummary, GlobalConfig } from './types/config'

/**
 * Determines the appropriate exit code based on execution results and CI configuration.
 * This is a standalone function for testing purposes.
 */
function determineExitCode(summary: ExecutionSummary, config: GlobalConfig): number {
	const { failed, total } = summary
	const ci = config.ci

	// If no failures, always exit with 0
	if (failed === 0) {
		return 0
	}

	// Check CI exit code options
	if (ci) {
		// strictExit: exit 1 if ANY failures occur
		if (ci.strictExit) {
			return 1
		}

		// failOn: exit 1 if failures exceed the threshold
		if (ci.failOn !== undefined && failed > ci.failOn) {
			return 1
		}

		// failOnPercentage: exit 1 if failure percentage exceeds the threshold
		if (ci.failOnPercentage !== undefined && total > 0) {
			const failurePercentage = (failed / total) * 100
			if (failurePercentage > ci.failOnPercentage) {
				return 1
			}
		}

		// If any CI option is set but thresholds not exceeded, exit 0
		if (ci.failOn !== undefined || ci.failOnPercentage !== undefined) {
			return 0
		}
	}

	// Default behavior: exit 1 if failures AND continueOnError is false
	return !config.continueOnError ? 1 : 0
}

/**
 * Creates a mock execution summary for testing
 */
function createSummary(total: number, failed: number): ExecutionSummary {
	return {
		total,
		successful: total - failed,
		failed,
		duration: 1000,
		results: [],
	}
}

describe('CI Exit Code', () => {
	describe('default behavior (no CI options)', () => {
		test('should exit 0 when no failures', () => {
			const summary = createSummary(10, 0)
			const config: GlobalConfig = {}
			expect(determineExitCode(summary, config)).toBe(0)
		})

		test('should exit 1 when failures exist and continueOnError is false', () => {
			const summary = createSummary(10, 2)
			const config: GlobalConfig = {}
			expect(determineExitCode(summary, config)).toBe(1)
		})

		test('should exit 0 when failures exist and continueOnError is true', () => {
			const summary = createSummary(10, 2)
			const config: GlobalConfig = { continueOnError: true }
			expect(determineExitCode(summary, config)).toBe(0)
		})
	})

	describe('--strict-exit flag', () => {
		test('should exit 1 when strictExit is true and any failures exist', () => {
			const summary = createSummary(10, 1)
			const config: GlobalConfig = { ci: { strictExit: true } }
			expect(determineExitCode(summary, config)).toBe(1)
		})

		test('should exit 0 when strictExit is true but no failures', () => {
			const summary = createSummary(10, 0)
			const config: GlobalConfig = { ci: { strictExit: true } }
			expect(determineExitCode(summary, config)).toBe(0)
		})

		test('should exit 1 when strictExit is true even with continueOnError', () => {
			const summary = createSummary(10, 1)
			const config: GlobalConfig = {
				continueOnError: true,
				ci: { strictExit: true },
			}
			expect(determineExitCode(summary, config)).toBe(1)
		})
	})

	describe('--fail-on threshold', () => {
		test('should exit 0 when failures are at or below threshold', () => {
			const summary = createSummary(10, 2)
			const config: GlobalConfig = { ci: { failOn: 2 } }
			expect(determineExitCode(summary, config)).toBe(0)
		})

		test('should exit 1 when failures exceed threshold', () => {
			const summary = createSummary(10, 3)
			const config: GlobalConfig = { ci: { failOn: 2 } }
			expect(determineExitCode(summary, config)).toBe(1)
		})

		test('should exit 0 when failOn is 0 and no failures', () => {
			const summary = createSummary(10, 0)
			const config: GlobalConfig = { ci: { failOn: 0 } }
			expect(determineExitCode(summary, config)).toBe(0)
		})

		test('should exit 1 when failOn is 0 and any failures exist', () => {
			const summary = createSummary(10, 1)
			const config: GlobalConfig = { ci: { failOn: 0 } }
			expect(determineExitCode(summary, config)).toBe(1)
		})
	})

	describe('--fail-on-percentage threshold', () => {
		test('should exit 0 when failure percentage is at or below threshold', () => {
			const summary = createSummary(100, 10)
			const config: GlobalConfig = { ci: { failOnPercentage: 10 } }
			expect(determineExitCode(summary, config)).toBe(0)
		})

		test('should exit 1 when failure percentage exceeds threshold', () => {
			const summary = createSummary(100, 11)
			const config: GlobalConfig = { ci: { failOnPercentage: 10 } }
			expect(determineExitCode(summary, config)).toBe(1)
		})

		test('should exit 0 when failOnPercentage is 50 and failures are 50%', () => {
			const summary = createSummary(10, 5)
			const config: GlobalConfig = { ci: { failOnPercentage: 50 } }
			expect(determineExitCode(summary, config)).toBe(0)
		})

		test('should exit 1 when failOnPercentage is 50 and failures are 51%', () => {
			const summary = createSummary(100, 51)
			const config: GlobalConfig = { ci: { failOnPercentage: 50 } }
			expect(determineExitCode(summary, config)).toBe(1)
		})

		test('should handle edge case with 0 total requests', () => {
			const summary = createSummary(0, 0)
			const config: GlobalConfig = { ci: { failOnPercentage: 10 } }
			expect(determineExitCode(summary, config)).toBe(0)
		})
	})

	describe('combined options', () => {
		test('strictExit takes precedence over failOn', () => {
			const summary = createSummary(10, 1)
			const config: GlobalConfig = {
				ci: { strictExit: true, failOn: 5 },
			}
			expect(determineExitCode(summary, config)).toBe(1)
		})

		test('strictExit takes precedence over failOnPercentage', () => {
			const summary = createSummary(100, 1)
			const config: GlobalConfig = {
				ci: { strictExit: true, failOnPercentage: 50 },
			}
			expect(determineExitCode(summary, config)).toBe(1)
		})

		test('failOn checked before failOnPercentage', () => {
			const summary = createSummary(100, 6) // 6% failure
			const config: GlobalConfig = {
				ci: { failOn: 5, failOnPercentage: 10 }, // Would pass percentage but fail count
			}
			expect(determineExitCode(summary, config)).toBe(1)
		})

		test('should exit 0 when all thresholds pass', () => {
			const summary = createSummary(100, 5) // 5% failure
			const config: GlobalConfig = {
				ci: { failOn: 5, failOnPercentage: 10 },
			}
			expect(determineExitCode(summary, config)).toBe(0)
		})
	})

	describe('CI option with continueOnError', () => {
		test('should still respect CI thresholds even with continueOnError', () => {
			const summary = createSummary(10, 3)
			const config: GlobalConfig = {
				continueOnError: true,
				ci: { failOn: 2 },
			}
			expect(determineExitCode(summary, config)).toBe(1)
		})

		test('should exit 0 when threshold not exceeded with continueOnError', () => {
			const summary = createSummary(10, 2)
			const config: GlobalConfig = {
				continueOnError: true,
				ci: { failOn: 2 },
			}
			expect(determineExitCode(summary, config)).toBe(0)
		})
	})
})
