// Re-export the official TanStack useForm hook as the canonical `useForm`.
// After migrating all forms to the official API we no longer need the local
// adapter. Keep the adapter file for now until we're confident everything
// has been migrated; it will be removed after this commit.
export { useForm } from '@tanstack/react-form';

// (Legacy) Keep the adapter type export available under a specific name in
// case other modules still import it directly.
export type { UseTanstackFormReturn as UseFormReturn } from './useTanstackForm';
