import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { AuthShell } from '@/components/AuthShell';

describe('AuthShell', () => {
  it('links Docs to the Cloud docs and Self-host it to the self-hosting guide', () => {
    render(<AuthShell footer>x</AuthShell>);
    expect(screen.getByRole('link', { name: 'Docs' })).toHaveAttribute(
      'href',
      'https://www.curl-runner.com/docs/features/cloud',
    );
    expect(screen.getByRole('link', { name: 'Self-host it' })).toHaveAttribute(
      'href',
      'https://www.curl-runner.com/docs/self-hosting',
    );
    expect(screen.queryByText('Status')).toBeNull();
  });
});
