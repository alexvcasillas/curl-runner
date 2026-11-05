import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { ConfirmDialog } from '@/components/ConfirmDialog';

function Harness({ onConfirm }: { onConfirm: () => Promise<void> }) {
  const [open, setOpen] = useState(false);
  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>
        Delete collection
      </button>
      <ConfirmDialog
        open={open}
        onClose={() => setOpen(false)}
        onConfirm={onConfirm}
        title="Delete Payments API"
        description="This deletes all 14 versions. You can’t undo it."
        confirmLabel="Delete collection"
        pendingLabel="Deleting…"
        confirmText="payments-api"
      />
    </>
  );
}

describe('ConfirmDialog', () => {
  it('unlocks only once the slug is typed', async () => {
    const onConfirm = vi.fn(async () => {});
    const user = userEvent.setup();
    render(<Harness onConfirm={onConfirm} />);
    await user.click(screen.getByRole('button', { name: 'Delete collection' }));
    const dialog = screen.getByRole('alertdialog', { name: 'Delete Payments API' });
    expect(dialog).toHaveAccessibleDescription('This deletes all 14 versions. You can’t undo it.');
    const confirm = screen.getAllByRole('button', { name: 'Delete collection' }).at(-1)!;
    expect(confirm).toBeDisabled();
    await user.type(screen.getByLabelText('Type payments-api to confirm'), 'payments');
    expect(confirm).toBeDisabled();
    await user.type(screen.getByLabelText('Type payments-api to confirm'), '-api');
    expect(confirm).toBeEnabled();
    await user.click(confirm);
    expect(onConfirm).toHaveBeenCalledOnce();
  });

  it('shows the error when the action fails', async () => {
    const user = userEvent.setup();
    render(
      <Harness
        onConfirm={async () => {
          throw new Error('Only team owners can do this.');
        }}
      />,
    );
    await user.click(screen.getByRole('button', { name: 'Delete collection' }));
    await user.type(screen.getByLabelText('Type payments-api to confirm'), 'payments-api{Enter}');
    expect(await screen.findByRole('alert')).toHaveTextContent('Only team owners can do this.');
  });

  it('closes on Escape and returns focus to the opener', async () => {
    const user = userEvent.setup();
    render(<Harness onConfirm={async () => {}} />);
    const opener = screen.getByRole('button', { name: 'Delete collection' });
    await user.click(opener);
    expect(screen.getByRole('alertdialog')).toBeInTheDocument();
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('alertdialog')).toBeNull();
    expect(opener).toHaveFocus();
  });
});
