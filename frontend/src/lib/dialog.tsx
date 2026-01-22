import React, { useCallback, useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';

type AlertOptions = {
  title?: string;
  okText?: string;
};

type ConfirmOptions = {
  title?: string;
  okText?: string;
  cancelText?: string;
  /**
   * If true, the primary action will use the "destructive" button variant.
   * Defaults to true for confirm dialogs.
   */
  destructive?: boolean;
};

/**
 * Internal handler references registered by `DialogProvider` on mount.
 * These are used by the exported `alert()` and `confirm()` helpers so they
 * can be called from anywhere (including outside React components).
 */
let _alertHandler: ((message: React.ReactNode, options?: AlertOptions) => Promise<void>) | null =
  null;
let _confirmHandler:
  | ((message: React.ReactNode, options?: ConfirmOptions) => Promise<boolean>)
  | null = null;

/**
 * Show a modal alert with a single "OK" button.
 * If the provider is not mounted, falls back to `window.alert`.
 */
export function alert(message: React.ReactNode, options?: AlertOptions): Promise<void> {
  if (_alertHandler) {
    return _alertHandler(message, options);
  }

  // Fallback for non-react environment / before provider mounts
  // Convert React nodes to strings when necessary.
  if (typeof message === 'string') {
    window.alert(message);
  } else {
    window.alert(String(message));
  }
  return Promise.resolve();
}

/**
 * Show a modal confirmation dialog with "OK" / "Cancel".
 * Resolves to `true` if user confirmed, `false` otherwise.
 * If the provider is not mounted, falls back to `window.confirm`.
 */
export function confirm(message: React.ReactNode, options?: ConfirmOptions): Promise<boolean> {
  if (_confirmHandler) {
    return _confirmHandler(message, options);
  }

  if (typeof message === 'string') {
    return Promise.resolve(window.confirm(message));
  } else {
    return Promise.resolve(window.confirm(String(message)));
  }
}

/**
 * Internal helper to register / clear handlers.
 * Called by the provider on mount/unmount.
 */
function setDialogHandlers(handlers?: {
  alert?: typeof _alertHandler;
  confirm?: typeof _confirmHandler;
}) {
  _alertHandler = handlers?.alert ?? null;
  _confirmHandler = handlers?.confirm ?? null;
}

/**
 * Simple hook for components that prefer to access dialog helpers via hooks.
 */
export function useDialog() {
  return {
    alert,
    confirm,
  };
}

/**
 * Internal representation of a queued dialog request.
 */
type DialogRequest =
  | {
      id: number;
      type: 'alert';
      message: React.ReactNode;
      title?: string;
      okText?: string;
      resolve: () => void;
    }
  | {
      id: number;
      type: 'confirm';
      message: React.ReactNode;
      title?: string;
      okText?: string;
      cancelText?: string;
      destructive?: boolean;
      resolve: (value: boolean) => void;
    };

/**
 * Provider component that renders modal dialogs and exposes handlers for
 * the `alert()` / `confirm()` helpers above.
 *
 * Wrap your application with this provider (e.g. in `main.tsx`) so dialogs
 * can be used from anywhere in the app.
 */
export function DialogProvider({ children }: { children: React.ReactNode }) {
  const { t } = useTranslation();
  const [queue, setQueue] = useState<DialogRequest[]>([]);
  const nextId = useRef(1);
  const primaryRef = useRef<HTMLButtonElement | null>(null);

  // Register handlers so global helpers work even outside React components.
  useEffect(() => {
    setDialogHandlers({
      alert: (message, options) =>
        new Promise<void>((resolve) => {
          const id = nextId.current++;
          setQueue((q) => [
            ...q,
            {
              id,
              type: 'alert',
              message,
              title: options?.title,
              okText: options?.okText ?? t('dialog.ok'),
              resolve,
            },
          ]);
        }),
      confirm: (message, options) =>
        new Promise<boolean>((resolve) => {
          const id = nextId.current++;
          setQueue((q) => [
            ...q,
            {
              id,
              type: 'confirm',
              message,
              title: options?.title,
              okText: options?.okText ?? t('dialog.yes'),
              cancelText: options?.cancelText ?? t('dialog.cancel'),
              destructive: options?.destructive ?? true,
              resolve,
            },
          ]);
        }),
    });

    // Clear handlers on unmount to avoid calling into unmounted provider.
    return () => {
      setDialogHandlers();
    };
  }, [t]);

  const current = queue.length > 0 ? queue[0] : null;

  // Prevent body scrolling while a dialog is open.
  useEffect(() => {
    if (!current) return;
    const previous = document.body.style.overflow;
    document.body.style.overflow = 'hidden';
    return () => {
      document.body.style.overflow = previous;
    };
  }, [current]);

  const closeCurrent = useCallback(() => {
    setQueue((q) => q.slice(1));
  }, []);

  const handleAlertOk = useCallback(() => {
    if (!current || current.type !== 'alert') return;
    current.resolve();
    closeCurrent();
  }, [current, closeCurrent]);

  const handleConfirm = useCallback(
    (value: boolean) => {
      if (!current || current.type !== 'confirm') return;
      current.resolve(value);
      closeCurrent();
    },
    [current, closeCurrent],
  );

  // Focus management and keyboard handling (Escape = cancel, Enter = confirm/ok)
  useEffect(() => {
    if (!current) return;
    // Focus primary button so Enter/Space act on the primary action.
    primaryRef.current?.focus();

    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        if (current.type === 'confirm') handleConfirm(false);
        else handleAlertOk();
      } else if (e.key === 'Enter') {
        if (current.type === 'confirm') handleConfirm(true);
        else handleAlertOk();
      }
    };

    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [current, handleAlertOk, handleConfirm]);

  return (
    <>
      {children}
      {current ? (
        <div
          className="fixed inset-0 z-[100] flex items-center justify-center bg-black/50 p-4"
          role="dialog"
          aria-modal="true"
          aria-labelledby={current.title ? `dialog-title-${current.id}` : undefined}
          aria-describedby={current ? `dialog-desc-${current.id}` : undefined}
        >
          <div className="w-full max-w-md rounded-lg bg-background p-6 shadow-lg">
            {current.title ? (
              <h2 id={`dialog-title-${current.id}`} className="text-lg font-semibold mb-2">
                {current.title}
              </h2>
            ) : null}

            <div id={`dialog-desc-${current.id}`} className="mb-4 text-sm text-muted-foreground">
              {current.message}
            </div>

            <div className="flex justify-end gap-2">
              {current.type === 'confirm' ? (
                <>
                  <Button variant="ghost" onClick={() => handleConfirm(false)}>
                    {current.cancelText}
                  </Button>
                  <Button
                    ref={primaryRef}
                    variant={current.destructive ? 'destructive' : 'default'}
                    onClick={() => handleConfirm(true)}
                  >
                    {current.okText}
                  </Button>
                </>
              ) : (
                <Button ref={primaryRef} onClick={handleAlertOk}>
                  {current.okText ?? t('dialog.ok')}
                </Button>
              )}
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
}
