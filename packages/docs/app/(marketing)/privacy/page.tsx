import type { Metadata } from 'next';
import { ContactLink, LEGAL_LINK, LegalPage, type LegalSection } from '@/components/site/LegalPage';
import { CLOUD_URL, PROCESSORS } from '@/lib/legal';
import { pageMetadata } from '@/lib/site';

export const metadata: Metadata = pageMetadata({
  title: 'Privacy policy',
  description:
    'What curl-runner and curl-runner Cloud collect about you, why, where it is kept and for how long.',
  path: '/privacy',
});

const sections: LegalSection[] = [
  {
    id: 'summary',
    title: 'Summary',
    body: (
      <ul>
        <li>
          The CLI and the studio run on your computer. They only talk to us when you use Cloud.
        </li>
        <li>
          Cloud keeps your email, your teams and the collections you publish, so it can share them.
        </li>
        <li>We don’t sell your data, show ads, or use your collections to train AI models.</li>
        <li>This website and Cloud set no tracking cookies.</li>
      </ul>
    ),
  },
  {
    id: 'website',
    title: 'This website',
    body: (
      <p className="m-0">
        curl-runner.com counts page views with Pulsora and Vercel Web Analytics. Neither sets
        cookies or stores anything in your browser. They record the page, the referrer, the browser,
        the device type and the country. Your IP address is used to work out the country and isn’t
        kept with the page views.
      </p>
    ),
  },
  {
    id: 'tools',
    title: 'The CLI and the studio',
    body: (
      <>
        <p className="m-0">
          Your collections, requests, responses and environment values stay on your computer. The
          CLI and the studio send nothing to us unless you sign in to a Cloud server, publish, pull,
          sync, or open a shared link.
        </p>
        <p className="m-0">
          When you open a shared collection, the values you type into its form, such as API keys,
          stay on your computer. They are never sent to Cloud.
        </p>
      </>
    ),
  },
  {
    id: 'account',
    title: 'Your Cloud account',
    body: (
      <>
        <p className="m-0">
          When you use curl-runner Cloud at{' '}
          <a href={CLOUD_URL} className={LEGAL_LINK}>
            cloud.curl-runner.com
          </a>
          , we keep:
        </p>
        <ul>
          <li>Your email address, and your name if GitHub or Google gives us one.</li>
          <li>
            For each sign-in method you link, the account ID and email address it reports. With
            GitHub we ask for your profile and email addresses (read:user and user:email). With
            Google we ask for openid, email and profile. We use them once, to sign you in, and don’t
            keep the access token.
          </li>
          <li>
            Your sessions: a name such as “Browser” or the CLI, when each started and when it was
            last used. We store a hash of each session token, not the token.
          </li>
          <li>Your teams, your role in each, and the invites you send or receive.</li>
          <li>
            Publish tokens your team creates: a name, who made it and when it was last used. Again,
            only a hash.
          </li>
          <li>
            The collections you publish: their files, every version, who or which token published
            each one, and the git commit and branch when CI sends them.
          </li>
          <li>
            Copies of the emails we send you: sign-in codes, invites, share notices, billing notices
            and, unless you turn them off, occasional product emails.
          </li>
        </ul>
        <p className="m-0">
          Cloud uses one cookie, <code>cr_session</code>, to keep you signed in, and short-lived
          cookies while you sign in. Your IP address is used to limit how often someone can try to
          sign in. It isn’t stored with your account. Actions taken in the admin panel are logged
          with the admin’s IP address.
        </p>
        <p className="m-0">
          The Cloud database runs on servers in Germany. Only the people who run curl-runner Cloud
          can reach it, and we look at your data only to give you support, when the law requires it,
          or to keep the service secure.
        </p>
      </>
    ),
  },
  {
    id: 'google',
    title: 'Google data',
    body: (
      <p className="m-0">
        Sign-in with Google only reads your name, email address and Google account ID, to sign you
        in. curl-runner’s use and transfer of information received from Google APIs adheres to the{' '}
        <a
          href="https://developers.google.com/terms/api-services-user-data-policy"
          className={LEGAL_LINK}
        >
          Google API Services User Data Policy
        </a>
        , including the Limited Use requirements. You can remove access at any time from your{' '}
        <a href="https://myaccount.google.com/permissions" className={LEGAL_LINK}>
          Google account permissions
        </a>
        .
      </p>
    ),
  },
  {
    id: 'sharing',
    title: 'Shared collections',
    body: (
      <>
        <p className="m-0">
          A team can share a collection publicly, or only with the email addresses and domains it
          lists. For a restricted collection we keep that list. When you open a shared collection
          while signed in, we record that you opened it, at most once a day. That’s how{' '}
          <em>Shared with me</em> finds it, and how the team sees whether the people on its list
          have opened it. Opening a public collection without signing in isn’t recorded.
        </p>
        <p className="m-0">
          If a team adds your email address to a collection, we keep it and may email you a link.
          The team decides who is on the list. Ask them, or us, to take you off.
        </p>
      </>
    ),
  },
  {
    id: 'billing',
    title: 'Billing',
    body: (
      <p className="m-0">
        Payments run through Stripe. Your card details, billing address and any tax ID go to Stripe,
        not to us. We keep the Stripe customer and subscription IDs, the billing interval and the
        subscription status.
      </p>
    ),
  },
  {
    id: 'processors',
    title: 'Service providers',
    body: (
      <>
        <p className="m-0">These companies process data for curl-runner.</p>
        <table className="w-full border-collapse text-left text-[15px]">
          <thead>
            <tr className="border-b border-line">
              <th scope="col" className="py-2 pr-4 font-semibold text-fg">
                Provider
              </th>
              <th scope="col" className="py-2 font-semibold text-fg">
                Purpose
              </th>
            </tr>
          </thead>
          <tbody>
            {PROCESSORS.map((p) => (
              <tr key={p.name} className="border-b border-hairline align-top">
                <td className="py-2 pr-4 text-fg">{p.name}</td>
                <td className="py-2">{p.purpose}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </>
    ),
  },
  {
    id: 'retention',
    title: 'Retention',
    body: (
      <ul>
        <li>
          Sessions end 90 days after they were last used, or when you sign out or revoke them.
        </li>
        <li>Sign-in codes expire after 10 minutes, and invites after 7 days.</li>
        <li>
          Collections and their versions stay until the team deletes them, or deletes the team.
        </li>
        <li>The admin activity log is kept for 2 years.</li>
        <li>
          Your account stays until you ask us to delete it. Some records, like copies of sent
          emails, stay in the database after you delete something. Write to us to have them erased.
        </li>
      </ul>
    ),
  },
  {
    id: 'rights',
    title: 'Your requests',
    body: (
      <p className="m-0">
        You can ask us to show, correct, export or delete the data we hold about you. Write to{' '}
        <ContactLink /> from the email address on your account, and we’ll answer within 30 days.
      </p>
    ),
  },
  {
    id: 'contact',
    title: 'Contact',
    body: (
      <p className="m-0">
        Questions about this policy go to <ContactLink />. If we change it in a way that matters,
        we’ll email Cloud account holders before the change takes effect.
      </p>
    ),
  },
];

export default function PrivacyPage() {
  return (
    <LegalPage
      title="Privacy policy"
      intro="What curl-runner collects, why, where it’s kept and for how long. It describes what the code does today."
      sections={sections}
    />
  );
}
