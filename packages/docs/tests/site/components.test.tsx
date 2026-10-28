import { fireEvent, render, screen } from '@testing-library/react';
import { renderToString } from 'react-dom/server';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { CopyButton } from '@/components/site/CopyButton';
import { SiteFooter } from '@/components/site/SiteFooter';
import { SiteHeader } from '@/components/site/SiteHeader';
import { Yaml } from '@/components/site/Yaml';

describe('Yaml', () => {
  it('colours keys with the accent and quoted strings with #F5A97F', async () => {
    const html = renderToString(await Yaml({ source: 'expect:\n  responseTime: "< 500"\n' }));
    expect(html).toMatch(/color:#22D3EE[^>]*>expect</);
    expect(html).toMatch(/color:#F5A97F[^>]*>"&#x3C; 500"/);
  });
});

describe('SiteHeader', () => {
  it('links the main nav and marks the active section', () => {
    render(<SiteHeader active="downloads" />);
    expect(screen.getByRole('link', { name: 'Docs' })).toHaveAttribute('href', '/docs');
    expect(screen.getByRole('link', { name: 'Features' })).toHaveAttribute(
      'href',
      '/features/parallel-execution',
    );
    expect(screen.getByRole('link', { name: 'Downloads' })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('link', { name: 'Changelog' })).not.toHaveAttribute('aria-current');
    expect(screen.getByRole('link', { name: 'GitHub' })).toHaveAttribute(
      'href',
      'https://github.com/alexvcasillas/curl-runner',
    );
    expect(screen.getByRole('link', { name: 'Get started' })).toHaveAttribute(
      'href',
      '/docs/quick-start',
    );
  });
});

describe('SiteFooter', () => {
  it('renders the three link columns', () => {
    render(<SiteFooter />);
    for (const heading of ['Product', 'Resources', 'Community']) {
      expect(screen.getByRole('heading', { name: heading })).toBeInTheDocument();
    }
    expect(screen.getByRole('link', { name: 'Tools database' })).toHaveAttribute(
      'href',
      '/tools-database',
    );
    expect(screen.getByRole('link', { name: 'Issues' })).toHaveAttribute(
      'href',
      'https://github.com/alexvcasillas/curl-runner/issues',
    );
  });
});

describe('CopyButton', () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('copies the text', async () => {
    const writeText = vi.fn().mockResolvedValue(undefined);
    vi.stubGlobal('navigator', { clipboard: { writeText } });
    render(<CopyButton text="hello" label="Copy greeting" />);
    fireEvent.click(screen.getByRole('button', { name: 'Copy greeting' }));
    expect(writeText).toHaveBeenCalledWith('hello');
    expect(await screen.findByText('Copied')).toBeInTheDocument();
  });

  it('swallows clipboard errors', async () => {
    const writeText = vi.fn().mockRejectedValue(new Error('denied'));
    vi.stubGlobal('navigator', { clipboard: { writeText } });
    render(<CopyButton text="hello" label="Copy greeting" />);
    fireEvent.click(screen.getByRole('button', { name: 'Copy greeting' }));
    await Promise.resolve();
    expect(writeText).toHaveBeenCalled();
    expect(screen.queryByText('Copied')).not.toBeInTheDocument();
  });
});
