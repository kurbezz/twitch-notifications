import { cn } from '@/lib/utils';

export interface ToggleProps {
  /**
   * Optional label displayed to the right of the control.
   */
  label?: string;
  /**
   * Current checked state.
   */
  checked: boolean;
  /**
   * Called when the control is toggled.
   */
  onChange: (checked: boolean) => void;
  /**
   * Disable control and interactions.
   */
  disabled?: boolean;
  /**
   * Optional aria-label for the underlying button (useful when `label` is not provided).
   */
  ariaLabel?: string;
  /**
   * Additional classes for the outer label wrapper.
   */
  className?: string;
}

/**
 * Reusable Toggle (switch) component.
 *
 * Visual and behavior intentionally mirrors the inline toggle used across the
 * Integrations/Settings UI. It is accessible (role="switch" + aria-checked)
 * and supports disabling interactivity.
 */
export function Toggle({
  label,
  checked,
  onChange,
  disabled = false,
  ariaLabel,
  className,
}: ToggleProps) {
  const buttonAriaLabel = ariaLabel ?? label ?? 'Toggle';

  return (
    <label className={cn('flex items-center gap-2 cursor-pointer', className)}>
      <button
        type="button"
        role="switch"
        aria-checked={checked}
        aria-label={buttonAriaLabel}
        disabled={disabled}
        onClick={() => {
          if (disabled) return;
          onChange(!checked);
        }}
        className={cn(
          'relative inline-flex h-5 w-9 flex-shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors',
          checked ? 'bg-twitch' : 'bg-muted',
          disabled && 'opacity-50 cursor-not-allowed',
        )}
      >
        <span
          className={cn(
            'pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow transition',
            checked ? 'translate-x-4' : 'translate-x-0',
          )}
        />
      </button>
      {label ? <span className="text-sm">{label}</span> : null}
    </label>
  );
}

export default Toggle;
