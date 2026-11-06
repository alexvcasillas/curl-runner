import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { AnnouncementBanner } from '@/components/AnnouncementBanner';

const banner = {
  message: 'Maintenance on Saturday at 02:00 UTC.',
  tone: 'warning' as const,
  link: 'https://status.example.com',
  version: 4,
};

afterEach(() => {
  localStorage.clear();
  vi.restoreAllMocks();
});

describe('AnnouncementBanner', () => {
  it('renders nothing without a banner', () => {
    const { container } = render(<AnnouncementBanner banner={null} />);
    expect(container).toBeEmptyDOMElement();
  });

  it('shows the message, the link in a new tab, and a dismiss button', () => {
    render(<AnnouncementBanner banner={banner} />);
    expect(screen.getByText(banner.message)).toBeInTheDocument();
    const link = screen.getByRole('link', { name: /learn more/i });
    expect(link).toHaveAttribute('href', 'https://status.example.com');
    expect(link).toHaveAttribute('target', '_blank');
    expect(link).toHaveAttribute('rel', expect.stringContaining('noreferrer'));
    expect(screen.getByRole('button', { name: 'Dismiss announcement' })).toBeInTheDocument();
  });

  it('has no link when there is none, and the tone picks the notice', () => {
    render(<AnnouncementBanner banner={{ ...banner, tone: 'info', link: null }} />);
    expect(screen.queryByRole('link')).toBeNull();
    expect(screen.getByRole('status')).toHaveTextContent(banner.message);
  });

  it('hides when dismissed and stays hidden for that version', async () => {
    const { unmount } = render(<AnnouncementBanner banner={banner} />);
    await userEvent.click(screen.getByRole('button', { name: 'Dismiss announcement' }));
    expect(screen.queryByText(banner.message)).toBeNull();
    expect(localStorage.getItem('cr_banner_dismissed=4')).toBe('1');
    unmount();
    render(<AnnouncementBanner banner={banner} />);
    expect(screen.queryByText(banner.message)).toBeNull();
  });

  it('comes back when the version changes', () => {
    localStorage.setItem('cr_banner_dismissed=4', '1');
    render(<AnnouncementBanner banner={{ ...banner, version: 5 }} />);
    expect(screen.getByText(banner.message)).toBeInTheDocument();
  });

  it('still works with storage blocked', async () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked');
    });
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new Error('blocked');
    });
    render(<AnnouncementBanner banner={banner} />);
    await userEvent.click(screen.getByRole('button', { name: 'Dismiss announcement' }));
    expect(screen.queryByText(banner.message)).toBeNull();
  });
});
