import { describe, expect, it } from 'vitest';
import { getFeatureBySlug } from '@/lib/features-data';

describe('test harness', () => {
  it('resolves the @/ alias and reads feature data', () => {
    expect(getFeatureBySlug('retry-mechanism')?.title).toBe('Retry Mechanism');
  });
});
