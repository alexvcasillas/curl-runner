import { ButtonLink } from './ButtonLink';
import { CheckIcon, ErrorIcon } from './icons';
import { Notice } from './Notice';

/** The 44px bordered badge above a title on the link and hand-off pages. */
export function Badge({ tone }: { tone: 'ok' | 'err' }) {
  return (
    <span
      aria-hidden="true"
      className="grid size-11 place-items-center rounded-[11px] border border-control bg-raised"
    >
      {tone === 'ok' ? (
        <CheckIcon size={18} className="text-success" />
      ) : (
        <ErrorIcon size={18} className="text-danger" />
      )}
    </span>
  );
}

export type AuthMessageProps = {
  title: string;
  body: string;
  actionLabel: string;
  actionHref: string;
  hint?: string;
};

/** A sign-in link or callback that didn't work: what happened, one way forward. */
export function AuthMessage({ title, body, actionLabel, actionHref, hint }: AuthMessageProps) {
  return (
    <div className="flex flex-col gap-5">
      <Badge tone="err" />
      <div className="flex flex-col gap-1.5">
        <h1 className="m-0 text-[24px] leading-8 font-semibold tracking-[-0.02em]">{title}</h1>
        <p className="m-0 text-fg-muted">{body}</p>
      </div>
      <ButtonLink href={actionHref} variant="primary" className="h-10! w-full text-[14px]!">
        {actionLabel}
      </ButtonLink>
      {hint ? <Notice tone="info">{hint}</Notice> : null}
    </div>
  );
}
