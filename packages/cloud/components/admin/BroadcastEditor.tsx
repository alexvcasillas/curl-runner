'use client';

import { useRouter } from 'next/navigation';
import { useCallback, useEffect, useId, useRef, useState } from 'react';
import {
  AUDIENCES,
  type Audience,
  audienceOf,
  type Broadcast,
  type DraftInput,
  MAX_BODY,
  MAX_SUBJECT,
  type Preview,
  peopleLabel,
  scheduledLabel,
  sendLabel,
} from '@/lib/admin-emails';
import { formatDate, relativeTime } from '@/lib/time';
import { Button } from '../Button';
import { Card, CardFooter, CardTitle } from '../Card';
import { Dialog } from '../Dialog';
import { inputClasses } from '../field-styles';
import { Notice } from '../Notice';
import { Segmented } from '../Segmented';
import type { EmailActions } from './email-actions';
import { ScheduleDialog } from './ScheduleDialog';

/** How long to wait after the last keystroke before saving the draft. */
const AUTOSAVE_DELAY = 800;
/** How long to wait after a change is saved, or a name typed, before asking for a new preview. */
const PREVIEW_DELAY = 400;
/** How often a sending broadcast reloads its counts. */
const SENDING_POLL = 5000;

const NO_PERM = 'You can read this page but not change it.';

type Form = DraftInput;
const same = (a: Form, b: Form) =>
  a.subject === b.subject && a.body_md === b.body_md && a.audience === b.audience;

type PreviewState =
  | { kind: 'loading'; last: Preview | null }
  | { kind: 'ok'; preview: Preview }
  | { kind: 'error'; message: string; last: Preview | null };

type SaveState = 'saved' | 'saving' | 'error';

function clockTime(d: Date): string {
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

/**
 * One broadcast: the draft, the live preview and the send controls.
 *
 * Saving and previewing. The draft saves itself 0.8 s after the last keystroke
 * (`PATCH`), and every action that needs the stored copy (preview, test, send,
 * schedule) saves first. The preview is the server's own rendering of the SAVED
 * draft (`GET …/preview?as_user=`), asked for once a save lands or the "Preview
 * as" name settles. The Markdown is never rendered here: the server's HTML goes
 * into a sandboxed frame with no scripts.
 */
export function BroadcastEditor({
  broadcast,
  initialCount,
  canEdit,
  now,
  actions,
  onUpdate,
}: {
  broadcast: Broadcast;
  initialCount: number | null;
  canEdit: boolean;
  now: string;
  actions: EmailActions;
  onUpdate: (b: Broadcast) => void;
}) {
  const router = useRouter();
  const id = broadcast.id;
  const open = broadcast.status === 'draft' || broadcast.status === 'scheduled';
  const editable = canEdit && open;
  const ids = useId();

  const [form, setForm] = useState<Form>({
    subject: broadcast.subject,
    body_md: broadcast.body_md,
    audience: broadcast.audience,
  });
  const formRef = useRef(form);
  formRef.current = form;
  const savedRef = useRef<Form>(form);
  const inflight = useRef<Promise<boolean> | null>(null);
  const [savedVersion, setSavedVersion] = useState(0);
  const [saveState, setSaveState] = useState<SaveState>('saved');
  const [savedAt, setSavedAt] = useState<Date | null>(null);

  const [counts, setCounts] = useState<Partial<Record<Audience, number>>>(
    initialCount === null ? {} : { [broadcast.audience]: initialCount },
  );
  const [asUser, setAsUser] = useState('');
  const [preview, setPreview] = useState<PreviewState>({ kind: 'loading', last: null });

  const [dialog, setDialog] = useState<'send' | 'schedule' | null>(null);
  const [confirmCount, setConfirmCount] = useState<number | null>(null);
  const [note, setNote] = useState<string | null>(null);
  const [problem, setProblem] = useState<string | null>(null);
  const [busy, setBusy] = useState<'test' | 'send' | null>(null);
  const [sendError, setSendError] = useState<string | null>(null);
  const [sending, setSending] = useState(false);

  const onUpdateRef = useRef(onUpdate);
  onUpdateRef.current = onUpdate;

  // ---- Saving ----

  /** Saves what is typed, if it differs from what is stored. True when the stored copy is current. */
  const flush = useCallback(async (): Promise<boolean> => {
    if (inflight.current) {
      await inflight.current;
    }
    const current = formRef.current;
    if (same(savedRef.current, current)) {
      return true;
    }
    const run = (async () => {
      setSaveState('saving');
      try {
        const r = await actions.save(id, current);
        if (!r.ok) {
          setSaveState('error');
          setProblem(r.message);
          return false;
        }
        savedRef.current = current;
        setProblem(null);
        setSaveState('saved');
        setSavedAt(new Date());
        setSavedVersion((v) => v + 1);
        onUpdateRef.current(r.broadcast);
        return true;
      } catch {
        setSaveState('error');
        setProblem('That didn’t save. Try again.');
        return false;
      }
    })();
    inflight.current = run;
    const ok = await run;
    inflight.current = null;
    return ok && same(savedRef.current, formRef.current);
  }, [actions, id]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: a save (savedVersion) re-arms the timer for edits made meanwhile
  useEffect(() => {
    if (!editable || same(savedRef.current, form)) {
      return;
    }
    const timer = setTimeout(() => void flush(), AUTOSAVE_DELAY);
    return () => clearTimeout(timer);
  }, [form, editable, flush, savedVersion]);

  // ---- The audience count ----

  const audience = form.audience;
  const known = counts[audience];
  useEffect(() => {
    if (known !== undefined) {
      return;
    }
    let live = true;
    actions
      .count(audience)
      .then((r) => {
        if (live && r.ok) {
          setCounts((c) => ({ ...c, [audience]: r.count }));
        }
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, [audience, known, actions]);

  // ---- The preview ----

  const request = useRef(0);
  // biome-ignore lint/correctness/useExhaustiveDependencies: asks again after every save (savedVersion) and when the name settles
  useEffect(() => {
    // Typed but not saved yet: the save that follows asks instead.
    if (!same(savedRef.current, formRef.current)) {
      return;
    }
    const timer = setTimeout(async () => {
      const mine = ++request.current;
      setPreview((p) => ({ kind: 'loading', last: p.kind === 'ok' ? p.preview : p.last }));
      try {
        const r = await actions.preview(id, asUser);
        if (mine !== request.current) {
          return;
        }
        setPreview((p) =>
          r.ok
            ? { kind: 'ok', preview: r.preview }
            : {
                kind: 'error',
                message: r.message,
                last: p.kind === 'ok' ? p.preview : p.last,
              },
        );
      } catch {
        if (mine === request.current) {
          setPreview((p) => ({
            kind: 'error',
            message: 'The preview didn’t load. Try again.',
            last: p.kind === 'ok' ? p.preview : p.last,
          }));
        }
      }
    }, PREVIEW_DELAY);
    return () => clearTimeout(timer);
  }, [savedVersion, asUser, id, actions]);

  // ---- A broadcast that is going out reloads its counts ----

  useEffect(() => {
    if (broadcast.status !== 'sending') {
      return;
    }
    const timer = setInterval(() => router.refresh(), SENDING_POLL);
    return () => clearInterval(timer);
  }, [broadcast.status, router]);

  // ---- Actions ----

  async function sendTest() {
    setBusy('test');
    setNote(null);
    setProblem(null);
    try {
      if (editable && !(await flush())) {
        return;
      }
      const r = await actions.test(id);
      if (r.ok) {
        setNote(`Test sent to ${r.to}. The subject starts with [Test].`);
      } else {
        setProblem(r.message);
      }
    } catch {
      setProblem('That didn’t work. Try again.');
    } finally {
      setBusy(null);
    }
  }

  /** Saves, then reads the exact audience size, then asks. */
  async function askSend() {
    setBusy('send');
    setNote(null);
    setProblem(null);
    try {
      if (!(await flush())) {
        return;
      }
      const r = await actions.count(audience);
      if (!r.ok) {
        setProblem(r.message);
        return;
      }
      setCounts((c) => ({ ...c, [audience]: r.count }));
      setConfirmCount(r.count);
      setSendError(null);
      setDialog('send');
    } catch {
      setProblem('That didn’t work. Try again.');
    } finally {
      setBusy(null);
    }
  }

  async function sendNow() {
    if (sending || confirmCount === null) {
      return;
    }
    setSending(true);
    setSendError(null);
    try {
      const r = await actions.send(id, null);
      if (r.ok) {
        setDialog(null);
        onUpdate(r.broadcast);
        setNote(
          `Sending to ${peopleLabel(confirmCount)}. It goes out in batches; this page shows how far it got.`,
        );
        router.refresh();
      } else {
        setSendError(r.message);
      }
    } catch {
      setSendError('That didn’t work. Try again.');
    } finally {
      setSending(false);
    }
  }

  async function openSchedule() {
    setBusy('send');
    setNote(null);
    setProblem(null);
    try {
      if (await flush()) {
        setDialog('schedule');
      }
    } finally {
      setBusy(null);
    }
  }

  // ---- What shows ----

  const filled = form.subject.trim() !== '' && form.body_md.trim() !== '';
  const hint = audienceOf(audience);
  const count = known ?? null;
  const reachable = count === null || count > 0;
  const canAct = editable && filled;
  const why = !canEdit ? NO_PERM : !filled ? 'Write a subject and a message first.' : undefined;
  const shown = preview.kind === 'ok' ? preview.preview : preview.last;

  const status =
    saveState === 'saving'
      ? 'Saving…'
      : saveState === 'error'
        ? 'Couldn’t save'
        : savedAt
          ? `Draft saved ${clockTime(savedAt)}`
          : `Last saved ${relativeTime(broadcast.updated_at, new Date(now))}`;

  return (
    <>
      <Card aria-label="Compose">
        <div className="flex min-h-11 flex-wrap items-center gap-2 border-b border-hairline px-4">
          <CardTitle className="mr-auto text-[14px] [overflow-wrap:anywhere]">
            {form.subject.trim() || 'New broadcast'}
          </CardTitle>
          {open ? (
            <span aria-live="polite" className="text-[12px] text-fg-subtle">
              {status}
            </span>
          ) : null}
        </div>
        <div className="flex flex-col gap-4 p-4">
          {broadcast.status === 'sending' ? (
            <Notice tone="info" role="status">
              Sending to {audienceOf(broadcast.audience).noun}:{' '}
              {broadcast.sent_count.toLocaleString('en-US')} queued so far,{' '}
              {broadcast.delivered.toLocaleString('en-US')} delivered. It can’t change now.
            </Notice>
          ) : null}
          {broadcast.status === 'sent' ? (
            <Notice tone="ok" role="status">
              Sent {broadcast.sent_at ? formatDate(broadcast.sent_at) : ''} to{' '}
              {audienceOf(broadcast.audience).noun}: {broadcast.delivered.toLocaleString('en-US')}{' '}
              delivered
              {broadcast.failed > 0 ? ` · ${broadcast.failed.toLocaleString('en-US')} failed` : ''}.
              It can’t change now.
            </Notice>
          ) : null}
          {broadcast.status === 'scheduled' && broadcast.scheduled_at ? (
            <Notice tone="info" role="status">
              Scheduled for {scheduledLabel(broadcast.scheduled_at)}. Edits you save go out with it.
            </Notice>
          ) : null}
          {note ? (
            <Notice tone="ok" role="status">
              {note}
            </Notice>
          ) : null}
          {problem ? (
            <Notice tone="err" role="alert">
              {problem}
            </Notice>
          ) : null}

          <div className="flex flex-col gap-2">
            <span className="text-[12px] text-fg-muted">Send to</span>
            <Segmented
              label="Audience"
              options={AUDIENCES.map((a) => ({ value: a.value, label: a.label }))}
              value={audience}
              onChange={(value) => {
                if (editable) {
                  setForm((f) => ({ ...f, audience: value }));
                }
              }}
              className={editable ? 'self-start' : 'pointer-events-none self-start opacity-60'}
            />
            <span className="text-[12px] leading-[18px] text-fg-subtle">
              {count === null
                ? `${hint.label}: ${hint.hint}.`
                : `${peopleLabel(count)}: ${hint.hint}.`}
            </span>
          </div>

          <div className="flex flex-col gap-1.5 text-[12px]">
            <label htmlFor={`${ids}-subject`} className="text-fg-muted">
              Subject
            </label>
            <input
              id={`${ids}-subject`}
              value={form.subject}
              maxLength={MAX_SUBJECT}
              disabled={!editable}
              autoComplete="off"
              onChange={(e) => setForm((f) => ({ ...f, subject: e.target.value }))}
              className={inputClasses(false)}
            />
          </div>

          <div className="flex flex-col gap-1.5 text-[12px]">
            <label htmlFor={`${ids}-body`} className="text-fg-muted">
              Message, in Markdown
            </label>
            <textarea
              id={`${ids}-body`}
              value={form.body_md}
              maxLength={MAX_BODY}
              disabled={!editable}
              rows={10}
              onChange={(e) => setForm((f) => ({ ...f, body_md: e.target.value }))}
              className={inputClasses(
                false,
                'h-[200px] resize-y py-2.5 font-mono text-[12px] leading-[18px]',
              )}
            />
            <span className="leading-[18px] text-fg-subtle">
              Variables: <code>{'{{first_name}}'}</code>, <code>{'{{team_name}}'}</code>,{' '}
              <code>{'{{plan}}'}</code>. A user in several teams gets one email, with their first
              team.
            </span>
          </div>
        </div>
        <CardFooter className="justify-between">
          <span>Sent from the server’s own address. Product emails skip anyone who opted out.</span>
          {open ? (
            <span className="ml-auto flex flex-wrap gap-2">
              <Button
                size="sm"
                disabled={!canAct}
                loading={busy === 'test'}
                title={why}
                onClick={sendTest}
              >
                Send test to me
              </Button>
              <Button
                size="sm"
                disabled={!canAct || busy !== null}
                title={why}
                onClick={openSchedule}
              >
                {broadcast.status === 'scheduled' ? 'Reschedule' : 'Schedule'}
              </Button>
              <Button
                size="sm"
                variant="primary"
                disabled={!canAct || !reachable}
                loading={busy === 'send'}
                title={why ?? (reachable ? undefined : 'No one is in this audience.')}
                onClick={askSend}
              >
                {sendLabel(count)}
              </Button>
            </span>
          ) : null}
        </CardFooter>
      </Card>

      <Card aria-label="Preview">
        <div className="flex min-h-11 flex-wrap items-center gap-3 border-b border-hairline px-4 py-1.5">
          <CardTitle className="mr-auto text-[14px]">Preview</CardTitle>
          {shown ? (
            <span className="text-[12px] text-fg-muted">
              As {shown.as_user.name ?? shown.as_user.email}
            </span>
          ) : null}
          <label className="flex items-center gap-2 text-[12px] whitespace-nowrap text-fg-muted">
            Preview as
            <input
              value={asUser}
              maxLength={254}
              autoComplete="off"
              placeholder="Email or user id. Empty is you."
              onChange={(e) => setAsUser(e.target.value)}
              className={inputClasses(false, 'w-[240px] max-w-full')}
            />
          </label>
        </div>
        <div className="flex flex-col gap-3 p-4">
          {preview.kind === 'error' ? (
            <Notice tone="err" role="alert">
              {preview.message}
            </Notice>
          ) : null}
          {shown && !shown.in_audience ? (
            <Notice tone="warn">
              {shown.as_user.email} isn’t in this audience, so they wouldn’t get this.
            </Notice>
          ) : null}
          {shown?.opted_out ? (
            <Notice tone="warn">
              {shown.as_user.email} turned off product emails, so they’d be skipped.
            </Notice>
          ) : null}
          {shown ? (
            <>
              <p className="m-0 text-[12px] text-fg-muted [overflow-wrap:anywhere]">
                Subject: {shown.subject}
              </p>
              <iframe
                title="Email preview"
                sandbox=""
                srcDoc={shown.html}
                // The email's own colours are for a light page; the dashboard's dark scheme
                // would otherwise give the frame a dark canvas under dark text.
                style={{ colorScheme: 'light', backgroundColor: '#fafafa' }}
                aria-busy={preview.kind === 'loading'}
                className={`box-border h-[460px] w-full rounded-[6px] border border-hairline ${preview.kind === 'loading' ? 'opacity-60' : ''}`}
              />
              <p className="m-0 text-[12px] text-fg-subtle">
                The server renders this from the saved draft. The unsubscribe link only works in
                real copies.
              </p>
            </>
          ) : preview.kind === 'loading' ? (
            <p role="status" className="m-0 text-[12px] text-fg-subtle">
              Loading the preview…
            </p>
          ) : null}
        </div>
      </Card>

      <Dialog
        open={dialog === 'send'}
        role="alertdialog"
        onClose={() => {
          if (!sending) {
            setDialog(null);
          }
        }}
        title={confirmCount === null ? 'Send now?' : `Send to ${peopleLabel(confirmCount)} now?`}
        description={`“${form.subject.trim()}”, to ${hint.noun}. You can’t unsend it.`}
        footer={
          <>
            <Button onClick={() => setDialog(null)} disabled={sending}>
              Cancel
            </Button>
            <Button variant="primary" loading={sending} onClick={sendNow}>
              {sending ? 'Sending…' : 'Send now'}
            </Button>
          </>
        }
      >
        <Notice tone="info">
          It goes out in batches, so a large audience takes a while. People who turned off product
          emails are already left out of this count.
        </Notice>
        {sendError ? (
          <Notice tone="err" role="alert">
            {sendError}
          </Notice>
        ) : null}
      </Dialog>

      <ScheduleDialog
        open={dialog === 'schedule'}
        onClose={() => setDialog(null)}
        rescheduling={broadcast.status === 'scheduled'}
        count={count}
        current={broadcast.scheduled_at}
        onSubmit={async (iso) => {
          const r = await actions.send(id, iso);
          if (r.ok) {
            onUpdate(r.broadcast);
            return { ok: true };
          }
          return r;
        }}
        onDone={() => {
          // The "Scheduled for …" notice above says it.
          router.refresh();
        }}
      />
    </>
  );
}
