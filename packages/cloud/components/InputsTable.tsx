import { cx } from '@/lib/cx';
import type { CollectionInput } from '@/lib/types';
import { Card, CardBody, CardFooter, CardTitle } from './Card';
import { EmptyState } from './EmptyState';
import { TableCell, TableHead, TableHeader, TableRow } from './Table';

function YesNo({ on }: { on: boolean }) {
  return <span className={on ? 'text-fg' : 'text-fg-subtle'}>{on ? 'Yes' : 'No'}</span>;
}

/** What a customer fills in, from the version's `cloud.inputs`. A secret's default is never shown. */
export function InputsTable({ name, inputs }: { name: string; inputs: CollectionInput[] }) {
  if (inputs.length === 0) {
    return (
      <EmptyState title="No inputs">
        {name} asks customers for nothing. Add inputs under cloud in curl-runner.yaml, and the next
        publish lists them here.
      </EmptyState>
    );
  }
  return (
    <Card aria-labelledby="inputs-title">
      <CardBody className="pb-4">
        <CardTitle id="inputs-title">Inputs</CardTitle>
        <p className="m-0 text-fg-muted leading-normal">
          Customers fill these in when they open {name}. Secret values stay on their machine and
          never reach Cloud. Change inputs in{' '}
          <span className="font-mono text-fg">curl-runner.yaml</span>, and the next publish updates
          them.
        </p>
      </CardBody>
      <div className="overflow-x-auto border-t border-hairline">
        <div role="table" aria-label="Inputs" style={{ minWidth: 820 }}>
          <TableHeader>
            <TableHead width={160}>Name</TableHead>
            <TableHead>Description</TableHead>
            <TableHead width={80}>Required</TableHead>
            <TableHead width={70}>Secret</TableHead>
            <TableHead width={70}>Locked</TableHead>
            <TableHead width={220}>Default</TableHead>
          </TableHeader>
          {inputs.map((input) => (
            <TableRow key={input.name}>
              <TableCell width={160} className="font-mono text-[12px] [overflow-wrap:anywhere]">
                {input.name}
              </TableCell>
              <TableCell className="text-fg-muted">{input.description}</TableCell>
              <TableCell width={80}>
                <YesNo on={input.required} />
              </TableCell>
              <TableCell width={70}>
                <YesNo on={input.secret} />
              </TableCell>
              <TableCell width={70}>
                <YesNo on={input.locked} />
              </TableCell>
              <TableCell
                width={220}
                className={cx(
                  'font-mono text-[12px] [overflow-wrap:anywhere]',
                  input.secret || input.default === null ? 'text-fg-subtle' : 'text-fg',
                )}
              >
                {input.secret ? 'Kept on each customer’s machine' : (input.default ?? 'None')}
              </TableCell>
            </TableRow>
          ))}
        </div>
      </div>
      <CardFooter>
        Required inputs come first in the customer’s form. Locked values are shown but can’t be
        changed.
      </CardFooter>
    </Card>
  );
}
