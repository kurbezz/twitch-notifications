/**
 * ESLint configuration for the frontend
 * - Using TypeScript parser and recommended TypeScript/React rules
 * - Keep this file in CommonJS format so ESLint loads it reliably
 */
module.exports = {
  root: true,
  env: {
    browser: true,
    es2021: true,
    node: true,
  },
  parser: '@typescript-eslint/parser',
  parserOptions: {
    ecmaVersion: 'latest',
    sourceType: 'module',
    ecmaFeatures: {
      jsx: true,
    },
  },
  settings: {
    react: {
      version: 'detect',
    },
  },
  plugins: ['@typescript-eslint', 'react', 'react-hooks'],
  extends: [
    'eslint:recommended',
    'plugin:@typescript-eslint/recommended',
    'plugin:react/recommended',
    'plugin:react-hooks/recommended',
  ],
  rules: {
    // Project preferences / sensible defaults
    '@typescript-eslint/explicit-module-boundary-types': 'off',
    '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
    // New JSX transform (React 17+) doesn't require React in scope
    'react/react-in-jsx-scope': 'off',
    // We use TypeScript for typings, so PropTypes are unnecessary
    'react/prop-types': 'off',
  },
  ignorePatterns: ['dist', 'build', 'node_modules'],
};