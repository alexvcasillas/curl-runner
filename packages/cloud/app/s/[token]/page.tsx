import type { Metadata } from 'next';
import { notFound } from 'next/navigation';
import { AnnouncementBanner } from '@/components/AnnouncementBanner';
import { FooterNote } from '@/components/DeadLink';
import { PublicShell } from '@/components/PublicShell';
import { ShareGate } from '@/components/ShareGate';
import { ShareLanding } from '@/components/ShareLanding';
import { studioLink } from '@/lib/deep-link';
import { env } from '@/lib/env';
import { ApiError } from '@/lib/errors';
import { getLanding, getPublicBanner, getViewer } from '@/lib/public-data';
import { relativeTimeInline } from '@/lib/time';
import { isShareToken } from '@/lib/tokens';

type Props = { params: Promise<{ token: string }> };

// Never the token: titles end up in history, shares and screenshots.
const ROBOTS = { index: false, follow: false };

export async function generateMetadata({ params }: Props): Promise<Metadata> {
  const { token } = await params;
  const fallback: Metadata = { title: 'Shared collection', robots: ROBOTS };
  if (!isShareToken(token)) {
    return fallback;
  }
  try {
    const landing = await getLanding(token);
    return landing.kind === 'open'
      ? { title: `${landing.info.name}, shared by ${landing.info.team}`, robots: ROBOTS }
      : fallback;
  } catch (e) {
    if (e instanceof ApiError) {
      return fallback;
    }
    throw e;
  }
}

export default async function SharePage({ params }: Props) {
  const { token } = await params;
  if (!isShareToken(token)) {
    notFound();
  }
  const landing = await getLanding(token);
  if (landing.kind === 'dead') {
    notFound();
  }
  // A visitor with no session needs no lookup; a stale one is signed out.
  const viewer = await getViewer();
  const announcement = <AnnouncementBanner banner={await getPublicBanner()} />;
  const returnTo = `/s/${token}`;

  if (landing.kind !== 'open') {
    return (
      <PublicShell signedInAs={viewer} footer={<FooterNote />} announcement={announcement}>
        <ShareGate
          returnTo={returnTo}
          limited={landing.kind === 'limited' ? landing.message : undefined}
          denied={
            landing.kind === 'denied' ? { message: landing.message, email: viewer } : undefined
          }
        />
      </PublicShell>
    );
  }

  const { info } = landing;
  const e = env();
  return (
    <PublicShell signedInAs={viewer} footer={<FooterNote />} announcement={announcement}>
      <ShareLanding
        team={info.team}
        name={info.name}
        description={info.description}
        access={info.access === 'restricted' ? 'restricted' : 'public'}
        requests={info.request_count}
        version={info.version}
        updated={relativeTimeInline(info.updated_at)}
        studioHref={studioLink(e.PUBLIC_API_URL, info.team_slug, info.collection, token)}
        command={`curl-runner pull ${e.PUBLIC_APP_URL}/s/${token}`}
      />
    </PublicShell>
  );
}
