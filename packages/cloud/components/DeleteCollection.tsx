'use client';

import { useState } from 'react';
import type { Source } from '@/lib/types';
import { Button } from './Button';
import { Card, CardBody, CardFooter, CardTitle } from './Card';
import { ConfirmDialog } from './ConfirmDialog';

export type DeleteResult = { ok: true } | { ok: false; message: string };

function consequence(source: Source, people: number): string {
  const who =
    people > 0
      ? `${people} ${people === 1 ? 'person' : 'people'} on the access list stop${people === 1 ? 's' : ''} getting updates.`
      : 'Anyone using the share link stops getting updates.';
  const again =
    source === 'git'
      ? 'The next CI publish creates it again, as a new collection with a new link.'
      : 'Teammates’ studios can share it again, as a new collection with a new link.';
  return `${who} Their local copies stay on their machines. ${again}`;
}

/** Delete, behind a confirm that types the slug. Owners only: others see why it isn't offered. */
export function DeleteCollection({
  name,
  slug,
  versions,
  people,
  source,
  owner = true,
  onDelete,
}: {
  name: string;
  slug: string;
  versions: number;
  /** Entries on the access list. */
  people: number;
  source: Source;
  owner?: boolean;
  onDelete: () => Promise<DeleteResult>;
}) {
  const [open, setOpen] = useState(false);
  return (
    <Card aria-labelledby="delete-title" className="border-danger-border">
      <CardBody className="flex-row flex-wrap items-center">
        <div className="flex min-w-[240px] flex-1 flex-col gap-1">
          <CardTitle id="delete-title" className="text-[14px] leading-5">
            Delete collection
          </CardTitle>
          <p className="m-0 text-fg-muted leading-normal">
            Deletes every version, the share link and the access list. Customers keep their local
            copy but get no more updates.
          </p>
        </div>
        {owner ? (
          <Button variant="danger" onClick={() => setOpen(true)}>
            Delete {name}
          </Button>
        ) : null}
      </CardBody>
      <CardFooter className="border-danger-border">
        Only team owners can delete a collection.
      </CardFooter>
      <ConfirmDialog
        open={open}
        onClose={() => setOpen(false)}
        title={`Delete ${name}`}
        description={`This deletes ${versions === 0 ? 'the collection' : versions === 1 ? 'its only version' : `all ${versions} versions`}, the share link and the access list. You can’t undo it.`}
        notice={{ tone: 'err', children: consequence(source, people) }}
        confirmText={slug}
        confirmLabel="Delete collection"
        pendingLabel="Deleting…"
        onConfirm={async () => {
          const result = await onDelete();
          if (!result.ok) {
            throw new Error(result.message);
          }
        }}
      />
    </Card>
  );
}
