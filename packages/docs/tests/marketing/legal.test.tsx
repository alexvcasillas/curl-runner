import { render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import PrivacyPage, { metadata as privacyMeta } from '@/app/(marketing)/privacy/page';
import TermsPage, { metadata as termsMeta } from '@/app/(marketing)/terms/page';
import { SiteFooter } from '@/components/site/SiteFooter';
import { CONTACT_EMAIL, PROCESSORS } from '@/lib/legal';
import { MARKETING_PAGES } from '@/lib/page-sources';

describe('privacy page', () => {
  it('has its title, canonical URL and a dated heading', () => {
    render(<PrivacyPage />);
    expect(screen.getByRole('heading', { level: 1, name: 'Privacy policy' })).toBeInTheDocument();
    expect(screen.getByText(/^Last updated /)).toBeInTheDocument();
    expect(privacyMeta.alternates?.canonical).toBe('https://www.curl-runner.com/privacy');
  });

  it('covers the website, the tools and Cloud, and how to ask about it', () => {
    render(<PrivacyPage />);
    for (const name of [
      'Summary',
      'This website',
      'The CLI and the studio',
      'Your Cloud account',
      'Shared collections',
      'Billing',
      'Service providers',
      'Retention',
      'Your requests',
      'Contact',
    ]) {
      expect(screen.getByRole('heading', { level: 2, name })).toBeInTheDocument();
    }
    const mail = screen.getAllByRole('link', { name: CONTACT_EMAIL })[0];
    expect(mail).toHaveAttribute('href', `mailto:${CONTACT_EMAIL}`);
    expect(CONTACT_EMAIL).toBe('hello@curl-runner.com');
  });

  it('lists every provider that processes data, with what it does', () => {
    render(<PrivacyPage />);
    const table = screen.getByRole('table');
    for (const p of PROCESSORS) {
      const row = within(table).getByRole('row', { name: new RegExp(p.name) });
      expect(row).toHaveTextContent(p.purpose);
    }
    expect(PROCESSORS.map((p) => p.name)).toEqual(
      expect.arrayContaining([
        'Hetzner',
        'Stripe',
        'Amazon SES',
        'GitHub',
        'Google',
        'Vercel',
        'Pulsora',
      ]),
    );
  });

  it('says what Google sign-in reads and that it follows the Limited Use rules', () => {
    render(<PrivacyPage />);
    expect(screen.getByText(/openid, email and profile/)).toBeInTheDocument();
    expect(
      screen.getByRole('link', { name: 'Google API Services User Data Policy' }),
    ).toHaveAttribute('href', 'https://developers.google.com/terms/api-services-user-data-policy');
  });
});

describe('terms page', () => {
  it('has its title, canonical URL and a dated heading', () => {
    render(<TermsPage />);
    expect(screen.getByRole('heading', { level: 1, name: 'Terms of service' })).toBeInTheDocument();
    expect(screen.getByText(/^Last updated /)).toBeInTheDocument();
    expect(termsMeta.alternates?.canonical).toBe('https://www.curl-runner.com/terms');
  });

  it('says cancelling runs to the end of the period, with no refunds', () => {
    render(<TermsPage />);
    expect(
      screen.getByRole('heading', { level: 2, name: 'Plans and billing' }),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/stays active until the end of the period you paid for/),
    ).toBeInTheDocument();
    expect(screen.getByText(/Payments are not refunded/)).toBeInTheDocument();
  });

  it('links the privacy policy and the contact address', () => {
    render(<TermsPage />);
    expect(screen.getAllByRole('link', { name: 'privacy policy' })[0]).toHaveAttribute(
      'href',
      '/privacy',
    );
    expect(screen.getAllByRole('link', { name: CONTACT_EMAIL })[0]).toHaveAttribute(
      'href',
      `mailto:${CONTACT_EMAIL}`,
    );
  });
});

describe('legal links', () => {
  it('the footer links both pages', () => {
    render(<SiteFooter />);
    expect(screen.getByRole('link', { name: 'Privacy' })).toHaveAttribute('href', '/privacy');
    expect(screen.getByRole('link', { name: 'Terms' })).toHaveAttribute('href', '/terms');
  });

  it('the sitemap includes both pages', () => {
    const paths = MARKETING_PAGES.map((p) => p.path);
    expect(paths).toContain('/privacy');
    expect(paths).toContain('/terms');
  });
});
