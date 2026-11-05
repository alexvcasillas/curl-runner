'use client';

import { createContext, useCallback, useContext, useEffect, useRef, useState } from 'react';
import { Button } from './Button';
import { ToneIcon } from './tones';

type ToastInput = {
  /** `ok` confirms a quiet action; `err` says it failed and offers a retry. */
  tone?: 'ok' | 'err';
  message: string;
  action?: { label: string; onClick: () => void };
};

type ToastItem = ToastInput & { id: number };

const ToastContext = createContext<((t: ToastInput) => void) | null>(null);

/** Shows toasts bottom right, for 4 seconds each. */
export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const nextId = useRef(0);

  const dismiss = useCallback((id: number) => {
    setToasts((all) => all.filter((t) => t.id !== id));
  }, []);

  const show = useCallback((t: ToastInput) => {
    nextId.current += 1;
    const id = nextId.current;
    setToasts((all) => [...all.slice(-2), { ...t, id }]);
  }, []);

  return (
    <ToastContext.Provider value={show}>
      {children}
      <div className="pointer-events-none fixed right-4 bottom-4 left-4 z-[60] flex flex-col items-end gap-3 sm:left-auto">
        {toasts.map((t) => (
          <Toast key={t.id} toast={t} onDone={() => dismiss(t.id)} />
        ))}
      </div>
    </ToastContext.Provider>
  );
}

function Toast({ toast, onDone }: { toast: ToastItem; onDone: () => void }) {
  useEffect(() => {
    const timer = setTimeout(onDone, 4000);
    return () => clearTimeout(timer);
  }, [onDone]);
  const tone = toast.tone ?? 'ok';
  return (
    <div
      role={tone === 'err' ? 'alert' : 'status'}
      className="pointer-events-auto flex max-w-full animate-rise items-center gap-2.5 rounded-[8px] border border-control bg-raised px-3.5 py-2.5 text-[13px] shadow-menu"
    >
      <ToneIcon tone={tone} />
      <span className="min-w-0">{toast.message}</span>
      {toast.action ? (
        <Button
          size="sm"
          className="ml-1"
          onClick={() => {
            toast.action?.onClick();
            onDone();
          }}
        >
          {toast.action.label}
        </Button>
      ) : null}
    </div>
  );
}

/** `toast({ message: 'Link copied' })`. Needs `ToastProvider` above it. */
export function useToast() {
  const show = useContext(ToastContext);
  if (!show) {
    throw new Error('useToast needs a ToastProvider');
  }
  return show;
}
