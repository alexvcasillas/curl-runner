import { fireEvent, render, screen, within } from '@testing-library/react';
import { renderToString } from 'react-dom/server';
import { describe, expect, it } from 'vitest';
import { InstallTabs } from '@/components/site/InstallTabs';

const CURL = 'curl -fsSL https://www.curl-runner.com/install.sh | bash';
const BREW = 'brew install alexvcasillas/tap/curl-runner';

describe('InstallTabs', () => {
  it('server-renders the default command without JS', () => {
    const html = renderToString(
      <InstallTabs methods={['curl', 'brew']} defaultId="curl" showNote={false} />,
    );
    expect(html).toContain(CURL);
  });

  it('switches command and selection when a tab is clicked', () => {
    render(<InstallTabs methods={['curl', 'brew']} defaultId="curl" showNote={false} />);
    const brewTab = screen.getByRole('tab', { name: 'Homebrew' });
    expect(brewTab).toHaveAttribute('aria-selected', 'false');
    fireEvent.click(brewTab);
    expect(brewTab).toHaveAttribute('aria-selected', 'true');
    expect(screen.getByRole('tab', { name: 'macOS / Linux' })).toHaveAttribute(
      'aria-selected',
      'false',
    );
    expect(screen.getByText(BREW)).toBeInTheDocument();
    expect(screen.queryByText(CURL)).not.toBeInTheDocument();
  });

  it('renders tabs inside a tablist', () => {
    render(<InstallTabs methods={['curl', 'brew']} defaultId="curl" showNote={false} />);
    const tablist = screen.getByRole('tablist');
    expect(within(tablist).getAllByRole('tab')).toHaveLength(2);
  });

  it('shows the note only when asked', () => {
    const { unmount } = render(<InstallTabs methods={['brew']} defaultId="brew" showNote={true} />);
    expect(screen.getByText(/Upgrade with brew upgrade curl-runner/)).toBeInTheDocument();
    unmount();
    render(<InstallTabs methods={['brew']} defaultId="brew" showNote={false} />);
    expect(screen.queryByText(/Upgrade with brew upgrade curl-runner/)).not.toBeInTheDocument();
  });

  it('has a labelled copy button', () => {
    render(<InstallTabs methods={['curl']} defaultId="curl" showNote={false} />);
    expect(screen.getByRole('button', { name: 'Copy install command' })).toBeInTheDocument();
  });
});
