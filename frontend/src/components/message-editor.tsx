import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { Edit, Copy, Save, Loader2 } from 'lucide-react';

export type PlaceholderInfo = {
  name: string;
  description: string;
  example: string;
};

export interface MessageEditorProps {
  label: string;
  description: string;
  value: string;
  placeholders: PlaceholderInfo[];
  onChange: (value: string) => void;
  onSave: (value?: string) => void;
  isSaving: boolean;
  // When false, entirely disables editing features and only shows read-only content.
  autoSave?: boolean;
  canEdit?: boolean;
}

/**
 * Reusable message editor used for notification templates.
 *
 * Props:
 * - label: Heading shown above the editor
 * - description: Small description text
 * - value: Controlled message template
 * - placeholders: list of available placeholders
 * - onChange: called when local text is changed (before saving)
 * - onSave: called to persist the template. Receives the current value as an argument to avoid stale reads when parent persists state.
 * - isSaving: shows saving state in the UI
 */
export function MessageEditor({
  label,
  description,
  value,
  placeholders,
  onChange,
  onSave,
  isSaving,
  autoSave = false,
  canEdit = true,
}: MessageEditorProps) {
  const [isEditing, setIsEditing] = useState(false);
  const [localValue, setLocalValue] = useState(value);
  const [copied, setCopied] = useState<string | null>(null);
  const { t } = useTranslation();

  useEffect(() => {
    setLocalValue(value);
  }, [value]);

  const handleSave = () => {
    // Normalize placeholders (convert {{name}} -> {name}) before saving to keep
    // server-side templates consistent and avoid mismatches.
    const normalized = normalizePlaceholders(localValue);
    // Ensure parent receives current (normalized) local value directly to avoid stale reads
    onChange(normalized);
    onSave(normalized);
    // reflect normalized value immediately in the editor
    setLocalValue(normalized);
    setIsEditing(false);
  };

  const handleCancel = () => {
    setLocalValue(value);
    if (autoSave) {
      // revert parent state when using auto-save mode
      onChange(value);
    }
    setIsEditing(false);
  };

  const insertPlaceholder = (placeholder: string) => {
    setLocalValue((prev) => prev + placeholder);
  };

  const normalizePlaceholders = (s: string) => {
    // collapse double-brace placeholders like {{name}} -> {name}
    // repeat until no double-brace patterns remain (e.g. {{{name}}} -> {{name}} -> {name})
    const re = /\{\{\s*([^{}]+?)\s*\}\}/g;
    let prev: string | null = null;
    let cur = s;
    while (prev !== cur) {
      prev = cur;
      cur = cur.replace(re, '{$1}');
    }
    return cur;
  };

  const copyText = async (text: string, label?: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(label ?? 'all');
      setTimeout(() => setCopied(null), 2000);
    } catch {
      // ignore clipboard errors silently
    }
  };

  return (
    <div className="rounded-lg border bg-card p-6">
      <div className="flex items-start justify-between">
        <div>
          <h3 className="font-semibold">{label}</h3>
          <p className="text-sm text-muted-foreground">{description}</p>
        </div>

        {!isEditing && (canEdit ?? true) && (
          <Button variant="ghost" size="sm" onClick={() => setIsEditing(true)}>
            <Edit className="h-4 w-4 mr-2" />
            {t('message_editor.edit')}
          </Button>
        )}
      </div>

      <div className="mt-4">
        {isEditing ? (
          <>
            <textarea
              className="w-full min-h-[140px] rounded-md border bg-background p-3 text-sm shadow-sm"
              value={localValue}
              onChange={(e) => {
                const v = e.target.value;
                setLocalValue(v);
                if (autoSave) {
                  const normalized = normalizePlaceholders(v);
                  onChange(normalized);
                  // Auto-save mode: persist immediately using normalized value to avoid
                  // parent state being out-of-date when saving.
                  onSave(normalized);
                }
              }}
            />

            <div className="mt-3 flex items-center justify-between gap-4">
              <div className="flex items-center gap-2 text-sm text-muted-foreground">
                <span>{t('message_editor.placeholders')}</span>
                {placeholders.map((ph) => (
                  <button
                    key={ph.name}
                    onClick={() => insertPlaceholder(ph.name)}
                    className={cn(
                      'rounded-md px-2 py-1 text-xs font-medium hover:bg-muted',
                      copied === ph.name ? 'bg-muted' : '',
                    )}
                    title={`${ph.description} — example: ${ph.example}`}
                  >
                    {ph.name}
                  </button>
                ))}
              </div>

              <div className="flex items-center gap-2">
                <Button variant="ghost" size="sm" onClick={handleCancel}>
                  {t('message_editor.cancel')}
                </Button>
                {!autoSave ? (
                  <Button variant="secondary" size="sm" onClick={handleSave} disabled={isSaving}>
                    {isSaving ? (
                      <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                    ) : (
                      <Save className="h-4 w-4 mr-2" />
                    )}
                    {t('message_editor.save')}
                  </Button>
                ) : (
                  <div className="text-sm text-muted-foreground flex items-center gap-2">
                    {isSaving ? (
                      <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                    ) : (
                      <Save className="h-4 w-4" />
                    )}
                    {isSaving ? t('message_editor.saving') : t('message_editor.saved')}
                  </div>
                )}
              </div>
            </div>
          </>
        ) : (
          <div className="flex items-start justify-between gap-6">
            <div className="prose max-w-none break-words text-sm text-muted-foreground">
              {value.split('\n').map((line, idx) => (
                <p key={idx} className="m-0">
                  {line}
                </p>
              ))}
            </div>

            <div className="ml-4 flex items-start gap-2">
              <Button
                variant="ghost"
                size="sm"
                onClick={() => copyText(value, 'all')}
                aria-label={t('message_editor.copy_template')}
              >
                <Copy className="h-4 w-4 mr-2" />
                {copied === 'all' ? t('message_editor.copied') : t('message_editor.copy')}
              </Button>
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default MessageEditor;
