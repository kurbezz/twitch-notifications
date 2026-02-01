import { useForm as tanUseForm } from '@tanstack/react-form';
import { useTanstackForm } from './useTanstackForm';

// Export the local adapter as the project-wide `useForm` so components using
// `register`/`setValue`/`getValues` keep working while we finish migration.
export { useTanstackForm as useForm };

// Also export the raw TanStack hook under a different name for direct usage
// in places where the official API is desired.
export { tanUseForm };

// Keep the adapter type export available if other modules reference it.
export type { UseTanstackFormReturn as UseFormReturn } from './useTanstackForm';
