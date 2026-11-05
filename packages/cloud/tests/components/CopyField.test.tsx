import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { CopyField } from '@/components/CopyField';

describe('CopyField', () => {
  it('copies the value and says so', async () => {
    const user = userEvent.setup();
    const writeText = vi.spyOn(navigator.clipboard, 'writeText');
    const onCopy = vi.fn();
    render(<CopyField variant="secret" label="token" value="crt_abc" onCopy={onCopy} />);
    await user.click(screen.getByRole('button', { name: 'Copy token' }));
    expect(writeText).toHaveBeenCalledWith('crt_abc');
    expect(onCopy).toHaveBeenCalledOnce();
    expect(screen.getByRole('button', { name: 'Copied token' })).toHaveTextContent('Copied');
    expect(screen.getByRole('status')).toHaveTextContent('Copied token');
  });
});
