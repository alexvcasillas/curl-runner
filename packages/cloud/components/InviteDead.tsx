import { buttonClasses } from './button-styles';
import { Card } from './Card';
import { PublicShell } from './PublicShell';

/** An invite that expired, was cancelled or was used. Rendered with a real 404. */
export function InviteDead() {
  return (
    <PublicShell cloud width="invite">
      <Card className="flex flex-col gap-3.5 rounded-[12px] px-7 py-8 max-sm:px-5">
        <h1 className="m-0 text-[20px] leading-7 font-semibold">This invite no longer works</h1>
        <p className="m-0 text-fg-muted">
          It expired, was cancelled, or someone already used it. Invites work for 7 days. Ask a team
          owner to send a new one.
        </p>
        <a href="/" className={buttonClasses('secondary', 'md', 'self-start')}>
          Go to the dashboard
        </a>
      </Card>
    </PublicShell>
  );
}
