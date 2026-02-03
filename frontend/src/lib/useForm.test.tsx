import React from 'react';
import { render, screen, act, waitFor } from '@testing-library/react';
import { describe, it, expect } from 'vitest';
import { useForm, useWatch } from './useForm';

interface TestCompProps {
  captureRef?: (f: unknown) => void;
}

function TestComp({ captureRef }: TestCompProps) {
  const form = useForm<{ a: string }>({ defaultValues: { a: '' } });
  if (captureRef) captureRef(form);
  // form is created and field is registered below
  const a = useWatch(form, 'a') ?? '';
  const Field = (form.Field ?? (() => null)) as React.ComponentType<unknown>;

  return (
    <div>
      <Field name="a">
        {(field: unknown) => {
          const typedField = field as {
            state?: { value: string };
            handleChange?: (v: string) => void;
            handleBlur?: () => void;
          };
          return (
            <input
              data-testid="field-input"
              value={typedField.state?.value ?? ''}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) =>
                typedField.handleChange?.(e.target.value)
              }
              onBlur={typedField.handleBlur}
            />
          );
        }}
      </Field>
      <div data-testid="value">{a}</div>
      <button
        data-testid="set-button"
        onClick={() => {
          const typedForm = form as {
            setFieldValue?: (field: string, value: string) => void;
            setValue: (field: string, value: string) => void;
          };
          if (typeof typedForm.setFieldValue === 'function') {
            typedForm.setFieldValue('a', 'bar');
          } else {
            typedForm.setValue('a', 'bar');
          }
        }}
      >
        Set
      </button>
    </div>
  );
}

describe('useWatch', () => {
  it('updates when setValue is called via UI', async () => {
    render(<TestComp />);
    expect(screen.getByTestId('value')).toHaveTextContent('');
    act(() => {
      screen.getByTestId('set-button').click();
    });
    await waitFor(() => {
      expect(screen.getByTestId('value')).toHaveTextContent('bar');
    });
  });

  it('updates when setValue is called programmatically', async () => {
    let ref: unknown;
    render(<TestComp captureRef={(f) => (ref = f)} />);
    expect(screen.getByTestId('value')).toHaveTextContent('');
    // debug: inspect form ref before programmatic setValue
    const typedRef = ref as {
      getValues?: () => unknown;
      setFieldValue?: (field: string, value: string) => void;
      setValue?: (field: string, value: string) => void;
      setFieldState?: (field: string, updater: (s: unknown) => unknown) => void;
    };
    console.log('ref keys before:', ref ? Object.keys(ref as Record<string, unknown>) : null);
    console.log(
      'ref.getValues before:',
      ref && typeof typedRef.getValues === 'function' ? typedRef.getValues() : undefined,
    );
    act(() => {
      console.log('calling ref.setFieldValue("a", "baz")');
      if (typeof typedRef?.setFieldValue === 'function') {
        typedRef.setFieldValue('a', 'baz');
      } else if (typeof typedRef?.setValue === 'function') {
        typedRef.setValue('a', 'baz');
      } else if (typeof typedRef?.setFieldState === 'function') {
        typedRef.setFieldState('a', (s: unknown) => ({ ...s, value: 'baz' }));
      }
    });
    console.log(
      'ref.getValues after (sync):',
      ref && typeof typedRef.getValues === 'function' ? typedRef.getValues() : undefined,
    );
    await waitFor(() => {
      expect(screen.getByTestId('value')).toHaveTextContent('baz');
    });
  });
});
