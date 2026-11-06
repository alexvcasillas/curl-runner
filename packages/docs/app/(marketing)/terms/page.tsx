import type { Metadata } from 'next';
import Link from 'next/link';
import { ContactLink, LEGAL_LINK, LegalPage, type LegalSection } from '@/components/site/LegalPage';
import { pageMetadata } from '@/lib/site';

export const metadata: Metadata = pageMetadata({
  title: 'Terms of service',
  description:
    'The terms for using curl-runner Cloud: accounts, plans and billing, acceptable use, your data and how the service can end.',
  path: '/terms',
});

function PrivacyLink() {
  return (
    <Link href="/privacy" className={LEGAL_LINK}>
      privacy policy
    </Link>
  );
}

const sections: LegalSection[] = [
  {
    id: 'service',
    title: 'The service',
    body: (
      <>
        <p className="m-0">
          curl-runner Cloud stores the API collections your team publishes and shares them with the
          people you choose. By creating an account or using Cloud you accept these terms. If you
          use it for a company, you accept them for that company.
        </p>
        <p className="m-0">
          The curl-runner CLI and studio are open source under the MIT license. That license, not
          these terms, covers the software itself, including when you run your own server.
        </p>
      </>
    ),
  },
  {
    id: 'account',
    title: 'Your account',
    body: (
      <ul>
        <li>Use an email address you control, and keep your devices and publish tokens safe.</li>
        <li>
          You are responsible for what happens under your account and your teams, including the
          members you invite and the people you share collections with.
        </li>
      </ul>
    ),
  },
  {
    id: 'billing',
    title: 'Plans and billing',
    body: (
      <ul>
        <li>The Free plan costs nothing and has the limits shown on the billing page.</li>
        <li>
          The Team plan is billed per member, monthly or yearly in advance, through Stripe. It
          renews until you cancel. Prices are in US dollars, before tax.
        </li>
        <li>
          Adding or removing a member changes the price, prorated on the next invoice. Customers you
          share collections with are never billed.
        </li>
        <li>
          You can switch between monthly and yearly, or cancel, at any time. A cancelled plan stays
          active until the end of the period you paid for. Payments are not refunded, except where
          the law requires it.
        </li>
        <li>
          If a payment fails and isn’t fixed within 14 days, or the plan ends, the team moves to
          Free. Nothing is deleted.
        </li>
      </ul>
    ),
  },
  {
    id: 'use',
    title: 'Acceptable use',
    body: (
      <>
        <p className="m-0">You agree not to:</p>
        <ul>
          <li>
            Publish secrets, such as API keys or passwords, in collections you share. Use inputs for
            them, so each person enters their own.
          </li>
          <li>Publish anything you don’t have the right to share, or anything illegal.</li>
          <li>
            Use collections to attack, overload or get into systems you aren’t allowed to test.
          </li>
          <li>Overload Cloud, probe it for weaknesses, or get around plan limits.</li>
        </ul>
      </>
    ),
  },
  {
    id: 'data',
    title: 'Your data',
    body: (
      <p className="m-0">
        The collections you publish belong to you. You give us permission to store, copy and serve
        them only to run Cloud for you and the people you share them with, as the <PrivacyLink />{' '}
        describes. You can pull every version while your account is active.
      </p>
    ),
  },
  {
    id: 'availability',
    title: 'Availability',
    body: (
      <p className="m-0">
        We work to keep Cloud fast and available, but we don’t guarantee it will be uninterrupted or
        free of errors. We may change, add or remove features. If we remove something you pay for,
        we’ll tell you in advance.
      </p>
    ),
  },
  {
    id: 'liability',
    title: 'Liability',
    body: (
      <p className="m-0">
        Cloud is provided “as is”. As far as the law allows, we aren’t liable for indirect or
        consequential losses, such as lost profits or lost data, and our total liability is limited
        to what you paid us in the 12 months before the claim. Nothing here limits liability that
        the law doesn’t let us limit.
      </p>
    ),
  },
  {
    id: 'ending',
    title: 'Ending',
    body: (
      <p className="m-0">
        You can delete your collections and teams at any time, and ask us to delete your account. We
        may suspend or close an account that breaks these terms or doesn’t pay, after notice where
        we reasonably can.
      </p>
    ),
  },
  {
    id: 'changes',
    title: 'Changes and contact',
    body: (
      <p className="m-0">
        We may update these terms. For changes that matter, we’ll email account holders before they
        take effect, and using Cloud after that means you accept them. Questions go to{' '}
        <ContactLink />.
      </p>
    ),
  },
];

export default function TermsPage() {
  return (
    <LegalPage
      title="Terms of service"
      intro="The agreement between you and curl-runner when you create a curl-runner Cloud account or use it."
      sections={sections}
    />
  );
}
