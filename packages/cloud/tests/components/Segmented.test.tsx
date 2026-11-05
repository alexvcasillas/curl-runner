import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { describe, expect, it } from 'vitest';
import { Segmented } from '@/components/Segmented';

function Harness() {
  const [v, setV] = useState<'annual' | 'monthly'>('annual');
  return (
    <Segmented
      label="Billing period"
      value={v}
      onChange={setV}
      options={[
        { value: 'annual', label: 'Annual' },
        { value: 'monthly', label: 'Monthly' },
      ]}
    />
  );
}

describe('Segmented', () => {
  it('is a radio group that arrow keys move through', async () => {
    const user = userEvent.setup();
    render(<Harness />);
    const annual = screen.getByRole('radio', { name: 'Annual' });
    expect(annual).toHaveAttribute('aria-checked', 'true');
    annual.focus();
    await user.keyboard('{ArrowRight}');
    const monthly = screen.getByRole('radio', { name: 'Monthly' });
    expect(monthly).toHaveAttribute('aria-checked', 'true');
    expect(monthly).toHaveFocus();
    expect(annual).toHaveAttribute('tabindex', '-1');
  });
});
