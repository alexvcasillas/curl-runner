import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { describe, expect, it, vi } from 'vitest';
import { Switch } from '@/components/Switch';

describe('Switch', () => {
  it('is a switch that reports its state', () => {
    render(<Switch checked aria-label="Gallery" onChange={() => {}} />);
    expect(screen.getByRole('switch', { name: 'Gallery' })).toHaveAttribute('aria-checked', 'true');
  });

  it('asks for the opposite on click', async () => {
    const onChange = vi.fn();
    const { rerender } = render(
      <Switch checked={false} aria-label="Gallery" onChange={onChange} />,
    );
    await userEvent.click(screen.getByRole('switch'));
    expect(onChange).toHaveBeenLastCalledWith(true);
    rerender(<Switch checked aria-label="Gallery" onChange={onChange} />);
    await userEvent.click(screen.getByRole('switch'));
    expect(onChange).toHaveBeenLastCalledWith(false);
  });

  it('toggles with Space and Enter once focused', async () => {
    const onChange = vi.fn();
    render(<Switch checked={false} aria-label="Gallery" onChange={onChange} />);
    await userEvent.tab();
    expect(screen.getByRole('switch')).toHaveFocus();
    await userEvent.keyboard(' ');
    await userEvent.keyboard('{Enter}');
    expect(onChange).toHaveBeenCalledTimes(2);
  });

  it('does nothing when disabled', async () => {
    const onChange = vi.fn();
    render(<Switch checked={false} disabled aria-label="Gallery" onChange={onChange} />);
    await userEvent.click(screen.getByRole('switch'));
    expect(screen.getByRole('switch')).toBeDisabled();
    expect(onChange).not.toHaveBeenCalled();
  });

  it('has a 44px hit area', () => {
    render(<Switch checked={false} aria-label="Gallery" onChange={() => {}} />);
    expect(screen.getByRole('switch').className).toContain('h-11');
  });
});
