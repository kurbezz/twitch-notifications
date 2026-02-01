import { useState, useCallback, useMemo } from 'react';
import { useForm as tanUseForm } from '@tanstack/react-form';

// Adapter that uses @tanstack/react-form when available, otherwise falls back
// to local state. Exposes a minimal register/handleSubmit/setValue/reset/getValues API.
export function useTanstackForm<T extends Record<string, unknown>>(opts?: {
  defaultValues?: T;
  onSubmit?: (values: T) => Promise<void> | void;
}) {
  const initial = useMemo(() => opts?.defaultValues ?? ({} as T), [opts?.defaultValues]);

  // Minimal shape we use from the TanStack instance. Keep methods optional
  // because the real instance shape may differ between library versions.
  type TanFormInstance = {
    getValue?: (name: keyof T) => unknown;
    getValues?: () => T;
    setValue?: (name: keyof T, value: unknown) => void;
    reset?: (values?: T) => void;
  };

  // create tanstack form instance (wrapped into our minimal shape)
  let tanForm: TanFormInstance | null = null;
  try {
    if (tanUseForm) {
      const raw = tanUseForm<T>({
        defaultValues: initial,
        onSubmit: (args: unknown) => {
          const value =
            args && typeof args === 'object' && 'value' in (args as Record<string, unknown>)
              ? (args as Record<string, unknown>)['value']
              : undefined;
          // call user callback with extracted value (or empty object)
          opts?.onSubmit?.((value ?? {}) as T);
        },
      });
      // adapt to our minimal interface
      const adapted = raw as unknown as TanFormInstance;
      tanForm = {
        getValue: typeof adapted.getValue === 'function' ? adapted.getValue.bind(raw) : undefined,
        getValues:
          typeof adapted.getValues === 'function' ? adapted.getValues.bind(raw) : undefined,
        setValue: typeof adapted.setValue === 'function' ? adapted.setValue.bind(raw) : undefined,
        reset: typeof adapted.reset === 'function' ? adapted.reset.bind(raw) : undefined,
      };
    }
  } catch {
    // if creating the tanstack form fails, we'll fallback to local state
    tanForm = null;
  }

  const [values, setValues] = useState<T>(initial);

  const register = useCallback(
    (name: keyof T) => {
      if (tanForm) {
        return {
          value: (tanForm.getValue ? tanForm.getValue(name) : undefined) ?? '',
          onChange: (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
            const target = e.target as HTMLInputElement;
            const val: unknown = target.type === 'checkbox' ? target.checked : target.value;
            if (tanForm.setValue) tanForm.setValue(name, val);
          },
        };
      }

      return {
        value: (values as unknown as Record<string, unknown>)[String(name)] ?? '',
        onChange: (e: React.ChangeEvent<HTMLInputElement | HTMLSelectElement>) => {
          const target = e.target as HTMLInputElement;
          const val: unknown = target.type === 'checkbox' ? target.checked : target.value;
          setValues((prev) => ({ ...prev, [name]: val }) as T);
        },
      };
    },
    [tanForm, values],
  );

  const handleSubmit = useCallback(
    (fn: (values: T) => Promise<void> | void) => {
      return async (e?: React.FormEvent) => {
        e?.preventDefault();
        if (tanForm && tanForm.getValues) {
          await fn(tanForm.getValues());
        } else {
          await fn(values);
        }
      };
    },
    [tanForm, values],
  );

  const setValue = useCallback(
    (name: keyof T, value: unknown) => {
      if (tanForm && typeof tanForm.setValue === 'function') {
        tanForm.setValue(name, value);
        return;
      }
      setValues((prev) => ({ ...prev, [name]: value }) as T);
    },
    [tanForm],
  );

  const reset = useCallback(
    (newValues?: T) => {
      if (tanForm && typeof tanForm.reset === 'function') {
        tanForm.reset(newValues ?? initial);
        return;
      }
      setValues(newValues ?? initial);
    },
    [tanForm, initial],
  );

  const getValues = useCallback(() => {
    if (tanForm && typeof tanForm.getValues === 'function') return tanForm.getValues();
    return values;
  }, [tanForm, values]);

  return { register, handleSubmit, setValue, reset, getValues } as const;
}

export type UseTanstackFormReturn<T> = ReturnType<typeof useTanstackForm<T>>;
