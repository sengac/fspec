import { createRequire } from 'module';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

// Get the directory where THIS wrapper file actually lives at runtime
const getWrapperDir = () => {
  try {
    // When bundled, use import.meta.url to find vendor directory relative to bundle
    const bundleFile = fileURLToPath(import.meta.url);
    const bundleDir = dirname(bundleFile);
    // Check if we're in the bundled location (dist/)
    if (bundleDir.includes('/dist')) {
      return join(bundleDir, 'vendor');
    }
    // Otherwise we're in development (vendor/)
    return bundleDir;
  } catch {
    // Fallback to __dirname
    return dirname(fileURLToPath(import.meta.url));
  }
};

const require = createRequire(import.meta.url);
const wrapperDir = getWrapperDir();
const dartModule = require(join(wrapperDir, 'tree-sitter-dart/bindings/node/index.js'));

export default dartModule;
