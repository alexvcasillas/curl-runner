'use client';

import { useState } from 'react';
import type { Source } from '@/lib/types';
import { Button } from './Button';
import { buttonClasses } from './button-styles';
import { Card, CardBody, CardTitle } from './Card';
import { Notice } from './Notice';
import { Segmented } from './Segmented';

export type SwitchResult = { ok: true } | { ok: false; message: string };

const OPTIONS: { value: Source; label: string }[] = [
  { value: 'git', label: 'Files' },
  { value: 'cloud', label: 'Cloud' },
];

const HELP: Record<Source, string> = {
  git: 'Files: each version is published from a folder, by CI, the CLI or the studio’s Publish. The studio can’t sync edits to it.',
  cloud:
    'Cloud: teammates sync edits from the studio, and every sync is a version. CI publishing is refused.',
};

/**
 * Where new versions come from. Picking the other side asks first, in place,
 * and says what the switch does to CI or to unsynced studio edits. Owners
 * only: members see the source as text.
 */
export function SourceSwitch({
  name,
  source,
  version,
  downloadHref,
  owner,
  onSwitch,
}: {
  name: string;
  source: Source;
  /** The current version, or null before the first. */
  version: number | null;
  /** The zip of the current version. */
  downloadHref: string;
  /** Whether the viewer owns the team; only owners can switch. */
  owner: boolean;
  onSwitch: (source: Source) => Promise<SwitchResult>;
}) {
  const [target, setTarget] = useState<Source | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function confirm() {
    if (!target || pending) {
      return;
    }
    setPending(true);
    setError(null);
    try {
      const result = await onSwitch(target);
      if (result.ok) {
        setTarget(null);
      } else {
        setError(result.message);
      }
    } catch {
      setError('That didn’t work. Try again.');
    } finally {
      setPending(false);
    }
  }

  return (
    <Card aria-labelledby="source-title">
      <CardBody>
        <CardTitle id="source-title">Source of truth</CardTitle>
        <p className="m-0 text-fg-muted leading-normal">
          Where new versions come from. Team owners can switch it, and the switch is the only way to
          change it.
        </p>
        {owner ? null : (
          <div className="flex flex-col gap-1">
            <span>Source: {OPTIONS.find((o) => o.value === source)?.label}</span>
            <span className="text-[12px] text-fg-muted">Only team owners can change this.</span>
          </div>
        )}
        {owner ? (
          <Segmented
            label="Source"
            options={OPTIONS}
            value={source}
            onChange={(next) => {
              setError(null);
              setTarget(next === source ? null : next);
            }}
            className="self-start"
          />
        ) : null}
        <p className="m-0 text-[12px] text-fg-muted">{HELP[source]}</p>

        {target ? (
          <Notice tone="warn" role="alert">
            <div className="flex flex-wrap items-center gap-3">
              <span className="min-w-[240px] flex-1">
                {target === 'cloud'
                  ? `CI publishes to ${name} will be refused with “synced from the studio”. Remove the publish step, or switch back to Files first.`
                  : version === null
                    ? 'Studio edits that aren’t in git yet will be overwritten by the next CI publish.'
                    : `v${version} stays current until the next CI publish replaces it. Studio edits that aren’t in git yet will be overwritten. Download them first to commit.`}
              </span>
              <span className="flex flex-wrap gap-2">
                {target === 'git' && version !== null ? (
                  <a href={downloadHref} className={buttonClasses('secondary', 'sm')}>
                    Download v{version} (.zip)
                  </a>
                ) : null}
                <Button size="sm" onClick={() => setTarget(null)} disabled={pending}>
                  Cancel
                </Button>
                <Button size="sm" variant="primary" onClick={confirm} loading={pending}>
                  {pending
                    ? 'Switching…'
                    : target === 'cloud'
                      ? 'Switch to Cloud'
                      : 'Switch to Files'}
                </Button>
              </span>
            </div>
          </Notice>
        ) : null}
        {error ? (
          <Notice tone="err" role="alert">
            {error}
          </Notice>
        ) : null}
      </CardBody>
    </Card>
  );
}
