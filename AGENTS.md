# AGENTS.md

This repository contains a Rust backend and a React/Vite frontend. Use this guide
to run builds/tests and to align with project conventions.

## Repository layout
- `backend/` - Rust Axum API + services, SQLite/SQLx, background workers
- `frontend/` - React + Vite app, Tailwind CSS, Vitest tests
- `nginx/` - Nginx config used by deployment
- `data/` - Local SQLite DB and runtime data

## Build, lint, and test commands

### Frontend (from `frontend/`)
- Install deps: `npm ci`
- Dev server: `npm run dev`
- Build: `npm run build`
- Preview build: `npm run preview`
- Lint: `npm run lint`
- Lint + fix: `npm run lint:fix`
- Tests (all): `npm run test`
- Tests (watch): `npm run test:watch`
- Single test file: `npx vitest src/lib/useForm.test.tsx`
- Single test by name: `npx vitest -t "useWatch"`
- Type check (CI): `npx tsc --noEmit`

### Backend (from `backend/`)
- Build: `cargo build`
- Run server: `cargo run`
- Format check (CI): `cargo fmt --all -- --check`
- Format (apply): `cargo fmt --all`
- Lint (CI): `cargo clippy --all-targets --all-features -- -D warnings`
- Tests (all): `cargo test --all-features`
- Single test by name: `cargo test <test_name>`
- Single module tests: `cargo test routes::auth::tests::test_name`

### Git hooks
- Husky pre-commit runs: `npx --no-install --prefix frontend lint-staged`
- lint-staged: Prettier + ESLint on `*.{ts,tsx,js,jsx}`; Prettier on `*.{json,md}`

## Code style guidelines

### General
- Prefer small, focused modules with explicit responsibilities.
- Keep functions single-purpose; avoid long handlers without helper functions.
- Favor clarity over cleverness; use explicit types and names.
- Do not add new deps without checking existing patterns.

### Frontend (React/TypeScript)

#### Imports
- Use path alias `@/` for app code (`@/components/...`, `@/lib/...`).
- Group imports: React/3rd-party first, then app modules, then styles.
- Use type-only imports (`import type { ... }`) when possible.

#### Formatting
- Prettier governs format: 2 spaces, 100 cols, single quotes, trailing commas.
- ESLint rules from `frontend/.eslintrc.cjs` are enforced in CI.
- Do not disable lint rules unless necessary and add a short reason.

#### Types
- `strict: true` in `tsconfig.json` - avoid `any` unless required.
- Prefer explicit generics for hooks/utilities when inference is ambiguous.
- Prefer `unknown` over `any` for error handling and narrow safely.

#### Naming conventions
- React components: `PascalCase`.
- Hooks: `useX`.
- Files: `kebab-case` for components, `PascalCase` for pages.
- CSS/Tailwind: use Tailwind classes; avoid inline styles unless dynamic.

#### Error handling
- Use `try/catch` for async flows, surface user-facing messages via UI helpers
  (e.g., `alert` from `frontend/src/lib/dialog.tsx`).
- Check for API error payloads before falling back to generic errors.
- Prefer `await` + `async` functions with clear error paths.

#### Data fetching/state
- React Query is used for API data and cache invalidation.
- `useForm` wrapper in `frontend/src/lib/useForm.ts` is the expected form API.
- Use `useAuth` for auth state rather than custom fetches.

### Backend (Rust)

#### Imports
- Group `std` imports first, then external crates, then local modules.
- Use explicit paths (`crate::...`) for internal modules.

#### Formatting
- `cargo fmt` is the canonical formatter (default Rustfmt settings).
- Do not hand-format; rely on Rustfmt in CI.

#### Naming conventions
- Modules/files: `snake_case`.
- Structs/enums/traits: `PascalCase`.
- Functions/vars: `snake_case`.
- Constants: `SCREAMING_SNAKE_CASE`.

#### Error handling
- Use `AppError` from `backend/src/error.rs` for HTTP responses.
- Prefer `?` for propagation; wrap with `anyhow` only when needed.
- Log meaningful context with `tracing` (`info!`, `warn!`, `error!`).
- Map external errors to `AppError` variants or `AppError::Internal`.

#### API patterns
- Route definitions live under `backend/src/routes/` and use Axum.
- Add new handlers to the appropriate router module and register in `routes/mod.rs`.
- Keep request/response DTOs close to handlers (same file) unless shared.

#### Database
- SQLx with SQLite; migrations in `backend/migrations/`.
- Use repository modules under `backend/src/db/repository/`.
- Keep models in `backend/src/db/models/`.

### Tests
- Frontend uses Vitest + Testing Library in `frontend/src/**`.
- Backend tests use standard `cargo test` with `#[tokio::test]` where needed.
- Name tests descriptively; prefer user-visible behavior over implementation details.

## Environment and config notes
- Frontend API URL via `VITE_API_URL` (see `frontend/vite.config.ts`).
- Backend reads config from env (`backend/src/config.rs`) and `.env` in `backend/`.
- Local SQLite DB exists at `data/app.db`.

## Cursor/Copilot rules
- No `.cursorrules`, `.cursor/rules/`, or `.github/copilot-instructions.md` found.

## Agent expectations
- Respect existing architecture boundaries between frontend and backend.
- Prefer smallest change that solves the task; avoid sweeping refactors.
- Update tests when behavior changes; keep CI commands green.
- Avoid adding secrets or committing `backend/.env` or local DB files.
