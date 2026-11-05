import { commitUrl, shortSha } from '@/lib/collections';
import { absoluteTime, relativeTime } from '@/lib/time';
import type { VersionSummary } from '@/lib/types';

/** "Published 2 hours ago by token ci from 3f9a2c1 on main". */
export function PublishedBy({
  version,
  lead,
}: {
  version: VersionSummary;
  /** What starts the sentence: "Published", or "v14 published". */
  lead: React.ReactNode;
}) {
  const href = commitUrl(version.git);
  const commit = version.git?.commit;
  const ref = version.git?.ref;
  return (
    <>
      {lead}{' '}
      <span title={absoluteTime(version.published_at)}>{relativeTime(version.published_at)}</span>{' '}
      by {version.publisher}
      {commit ? (
        <>
          {' '}
          from{' '}
          {href ? (
            <a href={href} className="font-mono text-[12px]">
              {shortSha(commit)}
            </a>
          ) : (
            <span className="font-mono text-[12px]">{shortSha(commit)}</span>
          )}
        </>
      ) : null}
      {ref ? (
        <>
          {' '}
          on <span className="font-mono text-[12px]">{ref}</span>
        </>
      ) : null}
    </>
  );
}
