import { buttonClasses } from './button-styles';
import { Card } from './Card';
import { PublicShell } from './PublicShell';

/** The page for a share link that is reset, expired or off. Rendered with a real 404. */
export function DeadLink() {
  return (
    <PublicShell footer={<FooterNote />}>
      <Card className="flex flex-col gap-3 px-7 py-8 max-sm:px-5">
        <span className="font-mono text-[12px] text-fg-subtle">404 Not Found</span>
        <h1 className="m-0 text-[20px] leading-7 font-semibold">This link no longer works</h1>
        <p className="m-0 text-fg-muted">
          It may have expired, been reset, or been turned off. Ask the team that shared it for a new
          one.
        </p>
        <a
          href="https://www.curl-runner.com"
          className={buttonClasses('secondary', 'md', 'self-start')}
        >
          What’s curl-runner?
        </a>
      </Card>
    </PublicShell>
  );
}

export function FooterNote() {
  return (
    <span>
      curl-runner runs HTTP requests from YAML.{' '}
      <a href="https://www.curl-runner.com" className="text-fg-muted hover:text-fg">
        Learn more
      </a>
    </span>
  );
}
