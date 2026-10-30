import { describe, expect, it } from 'vitest';
import { getAdjacentFeatures, getAllFeatures, getFeatureBySlug } from '@/lib/features-data';

const SIDEBAR_ORDER = [
  'parallel-execution',
  'response-validation',
  'retry-mechanism',
  'watch-mode',
  'ci-integration',
  'file-uploads',
  'output-formats',
  'response-storage',
  'conditional-execution',
  'performance-profiling',
  'response-diffing',
  'snapshots',
  'dry-run',
  'http2',
  'connection-pooling',
  'yaml-wizard',
  'validate',
  'convert',
  'env-files',
  'studio',
];

const BANNED_CLAIMS = /\d+x faster|up to \d+x|blazing|dramatically faster/i;

describe('features data', () => {
  const features = getAllFeatures();

  it('has all 20 features in sidebar order', () => {
    expect(features.map((f) => f.slug)).toEqual(SIDEBAR_ORDER);
  });

  it('has unique slugs', () => {
    expect(new Set(features.map((f) => f.slug)).size).toBe(features.length);
  });

  it('makes no invented speed claims', () => {
    for (const f of features) {
      expect(JSON.stringify(f), f.slug).not.toMatch(BANNED_CLAIMS);
    }
  });

  it('gives every feature complete copy', () => {
    for (const f of features) {
      expect(f.title, f.slug).not.toBe('');
      expect(f.headline, f.slug).toMatch(/[.?!]$/);
      expect(f.description.length, f.slug).toBeGreaterThan(40);
      expect(f.example.filename, f.slug).toMatch(/\.ya?ml$/);
      expect(f.example.yaml.trim(), f.slug).not.toBe('');
      expect(f.benefits, f.slug).toHaveLength(3);
      expect(f.goodFor, f.slug).toHaveLength(4);
      expect(f.keywords.length, f.slug).toBeGreaterThan(0);
    }
  });

  it('looks features up by slug', () => {
    expect(getFeatureBySlug('retry-mechanism')?.title).toBe('Retry Mechanism');
    expect(getFeatureBySlug('nope')).toBeNull();
  });

  it('links neighbours in sidebar order', () => {
    expect(getAdjacentFeatures('parallel-execution').prev).toBeNull();
    expect(getAdjacentFeatures('parallel-execution').next?.slug).toBe('response-validation');
    const retry = getAdjacentFeatures('retry-mechanism');
    expect(retry.prev?.slug).toBe('response-validation');
    expect(retry.next?.slug).toBe('watch-mode');
    expect(getAdjacentFeatures('env-files').next?.slug).toBe('studio');
    expect(getAdjacentFeatures('env-files').prev?.slug).toBe('convert');
    expect(getAdjacentFeatures('studio').next).toBeNull();
  });

  it('sharpens search titles and descriptions for the pages Search Console ranks', () => {
    const cases = [
      ['convert', 'Convert curl to YAML (and back) — curl-runner', 'convert curl'],
      ['retry-mechanism', 'Retry failed curl requests with backoff — curl-runner', 'retry'],
      ['file-uploads', 'Upload files with multipart form data — curl-runner', 'upload files'],
    ] as const;
    for (const [slug, seoTitle, query] of cases) {
      const f = getFeatureBySlug(slug);
      expect(f?.seoTitle, slug).toBe(seoTitle.replace(' — curl-runner', ''));
      expect(f?.description.length, slug).toBeLessThanOrEqual(150);
      expect(f?.description.toLowerCase(), slug).toContain(query);
    }
  });

  it('documents curl --retry on the retry page', () => {
    const f = getFeatureBySlug('retry-mechanism');
    expect(f?.fromCurl?.heading).toBe('From curl --retry');
    expect(f?.fromCurl?.href).toBe('https://curl.se/docs/manpage.html#--retry');
    expect(f?.example.yaml).toContain('retryableStatuses: [429, 500, 502, 503]');
  });
});
