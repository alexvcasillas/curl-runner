import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const startEmail = vi.fn();
const verifyCode = vi.fn();
const startOAuth = vi.fn();
vi.mock('@/app/sign-in/actions', () => ({ startEmail, verifyCode, startOAuth }));

const { SignInForm } = await import('@/app/sign-in/SignInForm');

beforeEach(() => {
  startEmail.mockReset();
  verifyCode.mockReset();
  startOAuth.mockReset();
});

const base = { returnTo: '/', client: 'web' as const };

describe('SignInForm', () => {
  it('shows no provider buttons when only email is on', () => {
    render(<SignInForm {...base} providers={[]} />);
    expect(screen.queryByText(/Continue with/)).toBeNull();
    expect(screen.getByRole('button', { name: 'Email me a code' })).toBeInTheDocument();
  });

  it('shows both providers when listed', () => {
    render(<SignInForm {...base} providers={['github', 'google']} />);
    expect(screen.getByRole('button', { name: /Continue with GitHub/ })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Continue with Google/ })).toBeInTheDocument();
  });

  it('goes from the email to the code step, then verifies the typed code', async () => {
    const user = userEvent.setup();
    startEmail.mockResolvedValue({ ok: true, email: 'ada@example.com' });
    verifyCode.mockResolvedValue({ ok: false, kind: 'wrong', message: 'x' });
    render(<SignInForm {...base} providers={[]} />);
    await user.type(screen.getByLabelText('Email'), 'ada@example.com');
    await user.click(screen.getByRole('button', { name: 'Email me a code' }));
    expect(await screen.findByText('Check your email')).toBeInTheDocument();
    expect(screen.getByText(/We sent a 6-digit code to/)).toHaveTextContent('ada@example.com');

    await user.click(screen.getByLabelText('Digit 1 of 6'));
    await user.keyboard('481903');
    await user.click(screen.getByRole('button', { name: 'Sign in' }));
    await waitFor(() => expect(verifyCode).toHaveBeenCalled());
    const fd = verifyCode.mock.calls[0][0] as FormData;
    expect(fd.get('code')).toBe('481903');
    expect(fd.get('email')).toBe('ada@example.com');
    expect(await screen.findByText(/That code isn’t right/)).toBeInTheDocument();
  });

  it('accepts a pasted code', async () => {
    const user = userEvent.setup();
    startEmail.mockResolvedValue({ ok: true, email: 'ada@example.com' });
    render(<SignInForm {...base} providers={[]} />);
    await user.type(screen.getByLabelText('Email'), 'ada@example.com');
    await user.click(screen.getByRole('button', { name: 'Email me a code' }));
    await user.click(await screen.findByLabelText('Digit 1 of 6'));
    await user.paste('481 903');
    expect(screen.getByLabelText('Digit 6 of 6')).toHaveValue('3');
  });

  it('shows the rate limit notice with the wait and disables the button', async () => {
    const user = userEvent.setup();
    startEmail.mockResolvedValue({ ok: false, kind: 'throttled', retryAfter: 720, message: 'x' });
    render(<SignInForm {...base} providers={['github', 'google']} />);
    await user.type(screen.getByLabelText('Email'), 'ada@example.com');
    await user.click(screen.getByRole('button', { name: 'Email me a code' }));
    const alert = await screen.findByRole('alert');
    expect(alert).toHaveTextContent('Try again in 12 minutes, or use GitHub or Google.');
    expect(screen.getByRole('button', { name: 'Email me a code' })).toBeDisabled();
  });

  it('offers a new code when the code expired', async () => {
    const user = userEvent.setup();
    startEmail.mockResolvedValue({ ok: true, email: 'ada@example.com' });
    verifyCode.mockResolvedValue({ ok: false, kind: 'expired', message: 'x' });
    render(<SignInForm {...base} providers={[]} />);
    await user.type(screen.getByLabelText('Email'), 'ada@example.com');
    await user.click(screen.getByRole('button', { name: 'Email me a code' }));
    await user.click(await screen.findByLabelText('Digit 1 of 6'));
    await user.keyboard('123456');
    await user.click(screen.getByRole('button', { name: 'Sign in' }));
    expect(await screen.findByText(/This code has expired/)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Sign in' })).toBeDisabled();
    await user.click(screen.getByRole('button', { name: 'Send a new code' }));
    expect(startEmail).toHaveBeenCalledTimes(2);
  });

  it('starts OAuth for the chosen provider with the destination', async () => {
    const user = userEvent.setup();
    render(<SignInForm returnTo="/t/acme" client="web" providers={['google']} />);
    await user.click(screen.getByRole('button', { name: /Continue with Google/ }));
    expect(startOAuth).toHaveBeenCalledWith('google', '/t/acme', 'web');
  });
});
