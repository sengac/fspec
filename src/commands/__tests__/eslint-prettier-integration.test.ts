/**
 * Feature: spec/features/add-eslint-and-prettier-for-code-quality.feature
 *
 * This test file validates the acceptance criteria defined in the feature file.
 * Scenarios in this test map directly to scenarios in the Gherkin feature.
 */

import { describe, it, expect } from 'vitest';
import { existsSync, readFileSync } from 'fs';
import { join } from 'path';

describe('Feature: Add ESLint and Prettier for code quality', () => {
  describe('Scenario: Run ESLint successfully with zero errors', () => {
    it('should have ESLint configured and all code passing with zero errors', () => {
      // @step Given ESLint is configured in eslint.config.js
      const eslintConfigPath = join(process.cwd(), 'eslint.config.js');

      // @step And All existing code violations have been fixed
      // @step When Developer runs 'npm run lint'
      // @step Then All TypeScript files should be checked
      // @step And Exit code should be 0
      // @step And Output should show zero errors and zero warnings

      // This test will FAIL until ESLint is actually configured
      expect(existsSync(eslintConfigPath)).toBe(true);
    });

    it('should have lint script in package.json', () => {
      // @step When Developer runs 'npm run lint'
      const packageJsonPath = join(process.cwd(), 'package.json');
      const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));

      // This test will FAIL until npm scripts include lint command
      expect(packageJson.scripts.lint).toBeDefined();
      expect(packageJson.scripts['lint:fix']).toBeDefined();
    });
  });

  describe('Scenario: Run Prettier successfully formatting all files', () => {
    it('should have Prettier configured with mindstrike style settings', () => {
      // @step Given Prettier is configured in .prettierrc
      const prettierConfigPath = join(process.cwd(), '.prettierrc');

      // @step And Code has mixed formatting styles
      // @step When Developer runs 'npm run format'
      // @step Then All files should be formatted with 2-space indentation
      // @step And Single quotes should be used consistently
      // @step And Exit code should be 0

      // This test will FAIL until Prettier is actually configured
      expect(existsSync(prettierConfigPath)).toBe(true);
    });

    it('should configure Prettier with project code style (2-space, single quotes, 80-char)', () => {
      // @step Then All files should be formatted with 2-space indentation
      // @step And Single quotes should be used consistently
      const prettierConfigPath = join(process.cwd(), '.prettierrc');

      if (existsSync(prettierConfigPath)) {
        const prettierConfig = JSON.parse(
          readFileSync(prettierConfigPath, 'utf-8')
        );

        // Validate expected configuration
        expect(prettierConfig.tabWidth).toBe(2);
        expect(prettierConfig.singleQuote).toBe(true);
        expect(prettierConfig.printWidth).toBe(80);
        expect(prettierConfig.semi).toBe(true);
        expect(prettierConfig.trailingComma).toBe('es5');
      } else {
        expect(existsSync(prettierConfigPath)).toBe(true);
      }
    });
  });

  describe('Scenario: Build completes successfully with linting integrated', () => {
    it('should have lint and format commands available for build integration', () => {
      // @step Given ESLint and Prettier are configured
      // @step And All code passes quality checks
      // @step When Developer runs 'npm run build'
      // @step Then TypeScript compilation should complete
      // @step And No lint errors should be reported
      // @step And Exit code should be 0

      const packageJsonPath = join(process.cwd(), 'package.json');
      const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));

      // This test will FAIL until lint and format scripts exist
      expect(packageJson.scripts.lint).toBeDefined();
      expect(packageJson.scripts.format).toBeDefined();
    });
  });

  describe('Scenario: Pre-commit hook runs ESLint and Prettier automatically', () => {
    it('should have Husky and lint-staged configured', () => {
      // @step Given Husky and lint-staged are configured
      // @step And Developer has staged files
      // @step When Developer runs git commit command
      // @step Then ESLint should run on staged files
      // @step And Prettier should format staged files
      // @step And Commit should succeed if no errors

      const packageJsonPath = join(process.cwd(), 'package.json');
      const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));

      // This test will FAIL until Husky and lint-staged are configured
      const hasHusky =
        packageJson.devDependencies && 'husky' in packageJson.devDependencies;
      const hasLintStaged =
        packageJson.devDependencies &&
        'lint-staged' in packageJson.devDependencies;

      expect(hasHusky).toBe(true);
      expect(hasLintStaged).toBe(true);
    });
  });

  describe('Scenario: ESLint catches common TypeScript errors during development', () => {
    it('should have ESLint configured with TypeScript plugin', () => {
      // @step Given ESLint is configured with TypeScript plugin
      // @step And Code has unused variables and missing types
      // @step When Developer runs 'npm run lint'
      // @step Then ESLint should report unused variable errors
      // @step And ESLint should report missing type errors
      // @step And Exit code should be 1

      const packageJsonPath = join(process.cwd(), 'package.json');
      const packageJson = JSON.parse(readFileSync(packageJsonPath, 'utf-8'));

      // This test will FAIL until TypeScript ESLint packages are installed
      const hasTypeScriptEslint =
        packageJson.devDependencies &&
        '@typescript-eslint/eslint-plugin' in packageJson.devDependencies &&
        '@typescript-eslint/parser' in packageJson.devDependencies;

      expect(hasTypeScriptEslint).toBe(true);
    });

    it('should have eslint.config.js with TypeScript rules', () => {
      const eslintConfigPath = join(process.cwd(), 'eslint.config.js');

      if (existsSync(eslintConfigPath)) {
        const eslintConfig = readFileSync(eslintConfigPath, 'utf-8');

        // Should reference TypeScript parser and plugin
        expect(eslintConfig).toContain('@typescript-eslint');
      } else {
        expect(existsSync(eslintConfigPath)).toBe(true);
      }
    });
  });

  describe('Scenario: Prettier formats code on save in IDE', () => {
    it('should have VSCode settings for format-on-save', () => {
      // @step Given VSCode is configured with format-on-save
      // @step And .vscode/settings.json has Prettier config
      // @step When Developer saves a TypeScript file
      // @step Then File should be formatted with 2-space indentation
      // @step And Single quotes should be used consistently
      // @step And Code style should match project standards

      const vscodeSettingsPath = join(
        process.cwd(),
        '.vscode',
        'settings.json'
      );

      // This test will FAIL until VSCode settings are configured
      expect(existsSync(vscodeSettingsPath)).toBe(true);
    });

    it('should configure VSCode with Prettier as default formatter', () => {
      const vscodeSettingsPath = join(
        process.cwd(),
        '.vscode',
        'settings.json'
      );

      if (existsSync(vscodeSettingsPath)) {
        const vscodeSettings = JSON.parse(
          readFileSync(vscodeSettingsPath, 'utf-8')
        );

        // Should have Prettier as default formatter and format on save
        expect(vscodeSettings['editor.formatOnSave']).toBe(true);
        expect(vscodeSettings['editor.defaultFormatter']).toContain('prettier');
      } else {
        expect(existsSync(vscodeSettingsPath)).toBe(true);
      }
    });
  });
});
