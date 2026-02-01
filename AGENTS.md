# AGENTS.md

## 1. Build, Lint, and Test Commands

### Frontend (React + Vite + TypeScript)

- **Working Directory**: `/frontend`
- **Build**: `npm run build` (runs `tsc` and `vite build`)
- **Dev Server**: `npm run dev` (runs `vite`)
- **Lint**: `npm run lint` (runs `eslint`)
- **Lint Fix**: `npm run lint:fix`
- **Tests**: No test scripts configured currently (checked `package.json`). If adding tests, use standard Vite/Vitest patterns.
- **Install Dependencies**: `npm install`

### Backend (Rust + Axum)

- **Working Directory**: `/backend`
- **Build**: `cargo build`
- **Run**: `cargo run` (starts the server)
- **Check**: `cargo check` (fast compile check)
- **Test**: `cargo test` (runs all tests)
  - **Single Test**: `cargo test test_name` (e.g., `cargo test tests::test_example`)
- **Lint**: `cargo clippy` (recommended for Rust)
- **Format**: `cargo fmt` (standard Rust formatter)
- **Database Migrations**: Uses `sqlx`. Ensure `DATABASE_URL` is set in `.env` or environment.
  - Run migrations: `sqlx migrate run`

## 2. Code Style & Conventions

### General
- **Path Handling**: ALWAYS use absolute paths for file operations. Combine project root with relative paths manually.
- **Environment**: This is a monorepo structure with `frontend/` and `backend/`. Respect the separation of concerns.

### Frontend (TypeScript/React)
- **Framework**: React 18, Vite, Tailwind CSS, shadcn/ui.
- **Language**: TypeScript (strict mode enabled).
- **Naming**: 
  - Components: PascalCase (e.g., `DashboardPage.tsx`, `Layout.tsx`).
  - Hooks: camelCase starting with `use` (e.g., `useAuth.ts`).
  - Functions/Variables: camelCase.
- **Imports**:
  - Use absolute imports with `@/` alias pointing to `src/` (e.g., `import { Layout } from '@/components/Layout';`).
  - Group imports: React/Third-party -> Local Components -> Hooks/Utils -> Types.
- **Styling**: Tailwind CSS utility classes. Use `cn()` helper (clsx + tailwind-merge) for conditional classes.
- **State Management**: `zustand` for global state, `react-query` (`@tanstack/react-query`) for server state.
- **Components**: Functional components with strict typing. `export function ComponentName() {}` is preferred over `const ComponentName = () => {}`.
- **Router**: `react-router-dom` v6.

### Backend (Rust)
- **Framework**: Axum (web), SQLx (database/sqlite), Tokio (async runtime).
- **Style**: Follow standard Rust idioms (Rustfmt, Clippy).
- **Naming**: 
  - Structs/Enums: PascalCase.
  - Functions/Variables/Modules: snake_case.
  - Constants: SCREAMING_SNAKE_CASE.
- **Error Handling**: Use `anyhow` for top-level application errors and `thiserror` for library/module-level errors. Custom `AppError` likely used for web responses.
- **Database**: SQLite with `sqlx`. Queries are async. Use compile-time checked queries (`sqlx::query!`) where possible.
- **Architecture**: Modular structure (`routes/`, `services/`, `db/`, `config/`).
- **Async**: Heavy use of `tokio` and `async/await`.

### Rules & Instructions
- **Modifying Code**: Analyze existing patterns first. If adding a new page, follow the `routes -> page component` pattern.
- **Dependencies**: Do not add new dependencies without checking if existing ones suffice (e.g., use `lucide-react` for icons).
- **Testing**: When writing new logic, check if there's a corresponding test file or create one if the infrastructure supports it (standard Rust `#[test]`).
