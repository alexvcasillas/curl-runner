import type { Screenshot } from '@/lib/studio';

/** Dashboard screenshots in `public/cloud/`, taken at 1280 CSS pixels wide on a 2x display. */
export const CLOUD_SCREENSHOTS = {
  collections: {
    src: '/cloud/cloud-collections.png',
    width: 2560,
    height: 1600,
    alt: 'The Collections page of the Acme team in the curl-runner Cloud dashboard, a table with the Payments API and Webhooks collections, their access setting, version and last publish.',
  },
  collection: {
    src: '/cloud/cloud-collection.png',
    width: 2560,
    height: 1600,
    alt: 'The Payments API collection page in the curl-runner Cloud dashboard, with a Versions tab listing v2 as current and v1 below it, each with who published it and a Changes button.',
  },
  sharing: {
    src: '/cloud/cloud-sharing.png',
    width: 2560,
    height: 2640,
    alt: 'The Sharing page of a collection in the curl-runner Cloud dashboard, with Off, Public and Restricted choices, the share link with a Copy link button, and an access list of two people.',
  },
  'tokens-created': {
    src: '/cloud/cloud-tokens-created.png',
    width: 2560,
    height: 1600,
    alt: 'The Token created dialog in the curl-runner Cloud dashboard, showing a new publish token with a Copy button and the GitHub Actions step to paste into a workflow file.',
  },
  members: {
    src: '/cloud/cloud-members.png',
    width: 2560,
    height: 1600,
    alt: 'The Members page of the Acme team in the curl-runner Cloud dashboard, with an invite form, two members and their roles, and one pending invite that expires in six days.',
  },
  landing: {
    src: '/cloud/cloud-landing.png',
    width: 2560,
    height: 1600,
    alt: 'The page a customer sees when they open a share link: the collection name, request count and version, an Open in curl-runner studio button and a curl-runner pull command.',
  },
} as const satisfies Record<string, Omit<Screenshot, 'title' | 'description'>>;

export type CloudScreenshotName = keyof typeof CLOUD_SCREENSHOTS;

export function cloudScreenshot(name: CloudScreenshotName): Screenshot {
  const shot = CLOUD_SCREENSHOTS[name];
  return { ...shot, title: shot.alt, description: shot.alt };
}
