import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { ShareAside } from '@/components/ShareAside';

describe('ShareAside', () => {
  it('previews the landing page in a new tab from the share link', () => {
    render(<ShareAside teamName="Acme" link="https://cloud.example.com/s/crsl_abc" />);
    const link = screen.getByRole('link', { name: 'Preview the page' });
    expect(link).toHaveAttribute('href', 'https://cloud.example.com/s/crsl_abc');
    expect(link).toHaveAttribute('target', '_blank');
    expect(link).toHaveAttribute('rel', expect.stringContaining('noreferrer'));
  });

  it('has no preview while sharing is off', () => {
    render(<ShareAside teamName="Acme" link="https://cloud.example.com/s/crsl_abc" off />);
    expect(screen.queryByRole('link', { name: 'Preview the page' })).toBeNull();
  });
});
