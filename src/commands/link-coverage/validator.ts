import { access } from 'fs/promises';
import { join } from 'path';

export interface LinkCoverageOptions {
  scenario: string;
  testFile?: string;
  testLines?: string;
  implFile?: string;
  implLines?: string;
  skipValidation?: boolean;
  skipStepValidation?: boolean;
  cwd?: string;
}

export function validateFlagCombinations(options: LinkCoverageOptions): void {
  const { testFile, testLines, implFile, implLines } = options;

  // Impl-only requires test-file
  if (implFile && !testFile) {
    throw new Error(
      '--test-file is required when adding implementation mappings\n' +
        'Implementation mappings attach to specific test mappings'
    );
  }

  // Test-only requires both test-file and test-lines
  if (testFile && !implFile && !testLines) {
    throw new Error(
      '--test-lines is required when linking test file\n' +
        'Example: --test-file src/__tests__/auth.test.ts --test-lines 45-62'
    );
  }

  // Impl mapping requires impl-lines
  if (implFile && !implLines) {
    throw new Error(
      '--impl-lines is required when linking implementation file\n' +
        'Example: --impl-file src/auth/login.ts --impl-lines 10,11,12'
    );
  }
}

export async function validateFiles(
  options: LinkCoverageOptions,
  warnings: string[]
): Promise<void> {
  const {
    testFile,
    implFile,
    skipValidation = false,
    cwd = process.cwd(),
  } = options;

  if (!skipValidation) {
    if (testFile) {
      await validateFileExists(join(cwd, testFile));
    }
    if (implFile) {
      await validateFileExists(join(cwd, implFile));
    }
  } else {
    // Add warnings for missing files when skipping validation
    if (testFile) {
      try {
        await access(join(cwd, testFile));
      } catch {
        warnings.push(`⚠️  File not found: ${testFile} (validation skipped)`);
      }
    }
    if (implFile) {
      try {
        await access(join(cwd, implFile));
      } catch {
        warnings.push(`⚠️  File not found: ${implFile} (validation skipped)`);
      }
    }
  }
}

async function validateFileExists(filePath: string): Promise<void> {
  try {
    await access(filePath);
  } catch {
    throw new Error(
      `File not found: ${filePath}\n` +
        'Suggestion: Ensure the file exists or use --skip-validation for forward planning'
    );
  }
}
