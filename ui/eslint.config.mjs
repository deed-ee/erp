import template from '@angular-eslint/eslint-plugin-template';
import templateParser from '@angular-eslint/template-parser';
import tseslint from 'typescript-eslint';

const syntaxRules = {
  '@typescript-eslint/no-explicit-any': 'error',
};

export default [
  {
    ignores: [
      'dist/**',
      '.angular/**',
      'test-results/**',
      'playwright-report/**',
    ],
  },
  {
    files: ['**/*.ts'],
    languageOptions: {
      parser: tseslint.parser,
    },
    plugins: { '@typescript-eslint': tseslint.plugin },
    rules: syntaxRules,
  },
  {
    files: ['src/**/*.ts'],
    languageOptions: {
      parserOptions: {
        projectService: true,
        tsconfigRootDir: import.meta.dirname,
      },
    },
    processor: template.processors['extract-inline-html'],
    rules: {
      '@typescript-eslint/no-non-null-assertion': 'error',
      '@typescript-eslint/no-floating-promises': 'error',
      '@typescript-eslint/only-throw-error': 'error',
    },
  },
  {
    files: ['src/**/*.html', '**/*inline-template*.html'],
    languageOptions: { parser: templateParser },
    plugins: { '@angular-eslint/template': template },
    rules: {
      '@angular-eslint/template/alt-text': 'error',
      '@angular-eslint/template/no-positive-tabindex': 'error',
    },
  },
];
