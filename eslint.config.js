import js from '@eslint/js';
import typescript from '@typescript-eslint/eslint-plugin';
import typescriptParser from '@typescript-eslint/parser';
import prettier from 'eslint-plugin-prettier';

export default [
  js.configs.recommended,
  {
    files: ['**/*.ts', '**/*.tsx'],
    ignores: ['extension/**'],
    languageOptions: {
      parser: typescriptParser,
      parserOptions: {
        ecmaVersion: 2022,
        sourceType: 'module',
        project: './tsconfig.json',
      },
      globals: {
        process: 'readonly',
        console: 'readonly',
        __dirname: 'readonly',
        __filename: 'readonly',
        Buffer: 'readonly',
        setTimeout: 'readonly',
        clearTimeout: 'readonly',
        setInterval: 'readonly',
        clearInterval: 'readonly',
      },
    },
    plugins: {
      '@typescript-eslint': typescript,
      prettier: prettier,
    },
    rules: {
      ...typescript.configs.recommended.rules,
      'prettier/prettier': 'error',
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/explicit-function-return-type': 'off',
      '@typescript-eslint/no-unused-vars': [
        'warn',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
        },
      ],
      'no-console': 'off',
      'no-undef': 'error',
    },
  },
  {
    files: ['src/commands/__tests__/extension-*.test.ts', 'src/__tests__/extension-*.test.ts'],
    languageOptions: {
      globals: {
        window: 'readonly',
        navigator: 'readonly',
        document: 'readonly',
        MessageEvent: 'readonly',
        EventListenerOrEventListenerObject: 'readonly',
        HTMLElement: 'readonly',
        HTMLInputElement: 'readonly',
        Event: 'readonly',
      },
    },
  },
  {
    files: ['extension/src/**/*.ts'],
    ignores: ['extension/src/**/__tests__/**'],
    languageOptions: {
      parser: typescriptParser,
      parserOptions: {
        ecmaVersion: 2022,
        sourceType: 'module',
        project: './extension/tsconfig.json',
      },
      globals: {
        // Node.js globals
        process: 'readonly',
        console: 'readonly',
        setTimeout: 'readonly',
        clearTimeout: 'readonly',
        setInterval: 'readonly',
        clearInterval: 'readonly',
        // Browser globals
        window: 'readonly',
        document: 'readonly',
        navigator: 'readonly',
        location: 'readonly',
        globalThis: 'readonly',
        MessageEvent: 'readonly',
        HTMLElement: 'readonly',
        HTMLInputElement: 'readonly',
        Event: 'readonly',
        NodeFilter: 'readonly',
        getComputedStyle: 'readonly',
        CheckVisibilityOptions: 'readonly',
        // Chrome extension globals
        chrome: 'readonly',
      },
    },
    plugins: {
      '@typescript-eslint': typescript,
      prettier: prettier,
    },
    rules: {
      ...typescript.configs.recommended.rules,
      'prettier/prettier': 'error',
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/explicit-function-return-type': 'off',
      '@typescript-eslint/no-unused-vars': [
        'warn',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
        },
      ],
      'no-console': 'off',
      'no-undef': 'error',
    },
  },
  {
    files: ['extension/src/**/__tests__/**/*.ts'],
    languageOptions: {
      parser: typescriptParser,
      parserOptions: {
        ecmaVersion: 2022,
        sourceType: 'module',
        project: './extension/tsconfig.json',
      },
      globals: {
        // Node.js globals
        process: 'readonly',
        console: 'readonly',
        setTimeout: 'readonly',
        clearTimeout: 'readonly',
        setInterval: 'readonly',
        clearInterval: 'readonly',
        __dirname: 'readonly',
        __filename: 'readonly',
        // Browser globals
        window: 'readonly',
        document: 'readonly',
        navigator: 'readonly',
        location: 'readonly',
        globalThis: 'readonly',
        MessageEvent: 'readonly',
        HTMLElement: 'readonly',
        HTMLInputElement: 'readonly',
        Event: 'readonly',
        NodeFilter: 'readonly',
        getComputedStyle: 'readonly',
        CheckVisibilityOptions: 'readonly',
        // Chrome extension globals
        chrome: 'readonly',
        // Vitest globals
        describe: 'readonly',
        it: 'readonly',
        expect: 'readonly',
        beforeAll: 'readonly',
        beforeEach: 'readonly',
        afterAll: 'readonly',
        afterEach: 'readonly',
        vi: 'readonly',
      },
    },
    plugins: {
      '@typescript-eslint': typescript,
      prettier: prettier,
    },
    rules: {
      ...typescript.configs.recommended.rules,
      'prettier/prettier': 'error',
      '@typescript-eslint/no-explicit-any': 'warn',
      '@typescript-eslint/explicit-function-return-type': 'off',
      '@typescript-eslint/no-unused-vars': [
        'warn',
        {
          argsIgnorePattern: '^_',
          varsIgnorePattern: '^_',
        },
      ],
      'no-console': 'off',
      'no-undef': 'error',
    },
  },
  {
    files: ['extension/host/**/*.mjs'],
    languageOptions: {
      ecmaVersion: 2022,
      sourceType: 'module',
      globals: {
        process: 'readonly',
        console: 'readonly',
        setTimeout: 'readonly',
        clearTimeout: 'readonly',
        setInterval: 'readonly',
        clearInterval: 'readonly',
        Buffer: 'readonly',
        URL: 'readonly',
      },
    },
    rules: {
      'no-undef': 'error',
      'no-unused-vars': ['warn', { argsIgnorePattern: '^_', varsIgnorePattern: '^_' }],
    },
  },
  {
    ignores: [
      'dist/**',
      'extension/dist/**',
      'node_modules/**',
      'coverage/**',
      'vendor/**',
      '*.config.js',
      'extension/vite.config.ts',
      'extension/vite.content.config.ts',
      'extension/vite.popup.config.ts',
      'extension/vitest.config.ts',
    ],
  },
];
