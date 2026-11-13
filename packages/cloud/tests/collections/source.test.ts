import { describe, expect, it } from 'vitest';
import { gitSummary, sourceKind } from '@/lib/collections';

describe('source kinds', () => {
  it('reads files with git details as Git, without as local files', () => {
    const git = { commit: '4f2a9c1aa0', ref: 'refs/heads/main', repo_url: null };
    expect(sourceKind('git', git)).toBe('git');
    expect(sourceKind('git', null)).toBe('files');
    expect(sourceKind('git', undefined)).toBe('files');
    expect(sourceKind('git', { commit: null, ref: null, repo_url: null })).toBe('files');
    expect(sourceKind('cloud', git)).toBe('cloud');
  });

  it('summarises repo, branch and commit', () => {
    expect(
      gitSummary({
        commit: '4f2a9c1aa0',
        ref: 'refs/heads/main',
        repo_url: 'https://github.com/acme/payments.git',
      }),
    ).toBe('acme/payments · main@4f2a9c1');
    expect(gitSummary({ commit: null, ref: 'main', repo_url: null })).toBe('main');
    expect(gitSummary({ commit: 'abcdef12', ref: null, repo_url: 'not a url' })).toBe('abcdef1');
    expect(gitSummary(null)).toBeNull();
  });
});
