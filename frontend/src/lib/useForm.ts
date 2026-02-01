import { useForm as tanUseForm } from '@tanstack/react-form';

// Provide a small wrapper around TanStack's `useForm` to present the simpler
// `setValue`/`getValues`/`reset` surface that the codebase expects while
// keeping full access to the underlying advanced API when needed.
export function useForm<T extends Record<string, unknown>>(opts?: {
  defaultValues?: T;
  onSubmit?: (values: T) => Promise<void> | void;
}) {
  // Do not specify the heavy generic parameters for the underlying hook —
  // let it infer. We adapt a minimal, typed surface on top.
  const raw = tanUseForm({
    defaultValues: opts?.defaultValues,
    onSubmit: (args: unknown) => {
      const value =
        args && typeof args === 'object' && 'value' in (args as Record<string, unknown>)
          ? (args as Record<string, unknown>)['value']
          : undefined;
      opts?.onSubmit?.((value ?? {}) as T);
    },
  }) as unknown as {
    setValue?: (name: keyof T, value: unknown) => void;
    getValues?: () => T;
    reset?: (values?: T) => void;
    // include index signature to allow spreading/forwarding any other runtime props
    [k: string]: unknown;
  };

  const setValue = raw.setValue ?? (() => {});
  const getValues = raw.getValues ?? (() => opts?.defaultValues ?? ({} as T));
  const reset = raw.reset ?? (() => {});

  return { ...raw, setValue, getValues, reset } as unknown;
}

export type UseFormReturn<T extends Record<string, unknown>> = ReturnType<typeof useForm<T>>;
