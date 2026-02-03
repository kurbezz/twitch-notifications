import { useForm as tanUseForm, type AnyFieldApi } from '@tanstack/react-form';
import { useEffect, useRef, useState, type ComponentType, type FormEvent } from 'react';

// Provide a small wrapper around TanStack's `useForm` to present the simpler
// `setValue`/`getValues`/`reset` surface that the codebase expects while
// keeping full access to the underlying advanced API when needed.
export function useForm<T extends Record<string, unknown>>(opts?: {
  defaultValues?: T;
  onSubmit?: (values: T) => Promise<void> | void;
}) {
  // Do not specify the heavy generic parameters for the underlying hook —
  // let it infer. We adapt a minimal, typed surface on top.
  const rawInstance = tanUseForm({
    defaultValues: opts?.defaultValues,
    onSubmit: (args: unknown) => {
      const value =
        args && typeof args === 'object' && 'value' in (args as Record<string, unknown>)
          ? (args as Record<string, unknown>)['value']
          : undefined;
      opts?.onSubmit?.((value ?? {}) as T);
    },
  }) as unknown;

  const raw = rawInstance as Record<string, unknown>;

  const setValue = (raw.setValue as (name: keyof T, value: unknown) => void) ?? (() => {});
  const getValues = (raw.getValues as () => T) ?? (() => opts?.defaultValues ?? ({} as T));
  const reset = (raw.reset as (values?: T) => void) ?? (() => {});

  return { ...raw, setValue, getValues, reset } as UseFormReturn<T>;
}

/**
 * Hook: useWatch
 *
 * Subscribe to a single field value of the given form and re-render when it changes.
 * Prefers using store.subscribe when available, otherwise falls back to periodic polling.
 * This implementation avoids conditional hook calls by using only hooks that are
 * always invoked in the same order.
 */
export function useWatch<T extends Record<string, unknown>, K extends keyof T>(
  form: UseFormReturn<T>,
  name: K,
): T[K] | undefined {
  // Read the current value from the form safely.
  // Prefer reading the individual field value when possible (more immediate),
  // then fall back to form-level value/state getters.
  const readValue = () => {
    try {
      const anyForm = form as unknown as Record<string, unknown>;

      // 1) Prefer per-field getter if available (immediate field-level read)
      const getFieldValue = anyForm.getFieldValue as ((field: string) => unknown) | undefined;
      if (typeof getFieldValue === 'function') {
        return getFieldValue(String(name)) as T[K];
      }

      // 2) Fallback to form-level getValues() if present
      const getValues = anyForm.getValues as (() => T) | undefined;
      if (typeof getValues === 'function') {
        const values = getValues();
        if (values && Object.prototype.hasOwnProperty.call(values, name as string)) {
          return (values as T)[name];
        }
      }

      // 3) Fallback to inspecting internal state object (if present)
      const state = (anyForm.state ?? anyForm.baseStore ?? undefined) as
        | { value?: Record<string, unknown> }
        | undefined;
      if (
        state &&
        state.value &&
        Object.prototype.hasOwnProperty.call(state.value, name as string)
      ) {
        return (state.value as Record<string, unknown>)[name as string] as T[K];
      }

      // 4) As last-resort, try getFieldMeta().value (some adapters expose field meta)
      const getFieldMeta = anyForm.getFieldMeta as ((field: string) => unknown) | undefined;
      if (typeof getFieldMeta === 'function') {
        const meta = getFieldMeta(String(name)) as Record<string, unknown> | undefined;
        if (meta && Object.prototype.hasOwnProperty.call(meta, 'value')) {
          return meta.value as T[K];
        }
      }

      // If nothing found, return undefined
      return undefined as unknown as T[K];
    } catch {
      return undefined as unknown as T[K];
    }
  };

  // Keep a ref to the reader to avoid capturing a stale closure in the effect
  const readerRef = useRef<() => T[K] | undefined>(() => readValue());

  // Initialize state with the current value
  const [value, setValue] = useState<T[K] | undefined>(() => readerRef.current());

  const prevRef = useRef<T[K] | undefined>(value);

  // Update readerRef each render so runner always reads latest data
  readerRef.current = () => readValue();

  useEffect(() => {
    const runner = () => {
      const next = readerRef.current();
      if (next !== prevRef.current) {
        prevRef.current = next;
        setValue(next);
      }
    };

    // initial sync
    runner();

    const s = (form as unknown as { store?: unknown }).store;
    const hasSubscribe = s && typeof (s as { subscribe?: unknown }).subscribe === 'function';
    let unsub: (() => void) | undefined;

    if (hasSubscribe) {
      // Subscribe to store if available
      unsub = (s as { subscribe: (fn: () => void) => () => void }).subscribe(runner);
    } else {
      // Fallback: poll periodically
      const id = setInterval(runner, 200);
      unsub = () => clearInterval(id);
    }

    return () => {
      if (unsub) unsub();
    };
  }, [form, name]);

  return value;
}

export type UseFormReturn<T extends Record<string, unknown>> = {
  setValue: (name: keyof T, value: unknown) => void;
  getValues: () => T;
  reset: (values?: T) => void;
  /**
   * Helper to trigger submit programmatically. Optional because some callers
   * use native form `onSubmit` and call `form.handleSubmit()` themselves.
   */
  handleSubmit?: (e?: FormEvent) => Promise<void> | void;
  /**
   * React components provided by TanStack Form for rendering fields and subscribing
   * to form state. These are optional and typed as ComponentType to stay flexible.
   */
  Field?: ComponentType<{
    name?: keyof T | string;
    children?: (field: AnyFieldApi) => unknown;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    validators?: Record<string, (args: { value: any }) => string | undefined>;
  }>;
  Subscribe?: ComponentType<{
    selector?: (state: unknown) => unknown;
    children?: (value: unknown) => unknown;
  }>;
  /**
   * The underlying TanStack Form store (if available). Exposed so consumers
   * can subscribe or use `useStore(form.store, selector)` if desired.
   */
  store?: unknown;
} & Record<string, unknown>;
