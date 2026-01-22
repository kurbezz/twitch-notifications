import type { ChangeEvent } from 'react';

interface ToggleSwitchProps {
  label: string;
  description?: string;
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}

export default function ToggleSwitch({
  label,
  description,
  checked,
  onChange,
  disabled = false,
}: ToggleSwitchProps) {
  return (
    <div className="flex items-start justify-between rounded-lg border p-4">
      <div>
        <h3 className="font-semibold">{label}</h3>
        {description && <p className="text-sm text-muted-foreground">{description}</p>}
      </div>
      <div>
        <input
          type="checkbox"
          role="switch"
          aria-checked={checked}
          aria-label={label}
          className="h-6 w-6 rounded border bg-background accent-twitch"
          checked={checked}
          onChange={(e: ChangeEvent<HTMLInputElement>) => onChange(e.target.checked)}
          disabled={disabled}
        />
      </div>
    </div>
  );
}
