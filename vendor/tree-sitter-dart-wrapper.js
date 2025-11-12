import { createRequire } from 'module';
import { fileURLToPath } from 'url';
import { dirname, join } from 'path';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const require = createRequire(import.meta.url);

const dartModule = require(join(__dirname, 'tree-sitter-dart/bindings/node/index.js'));

export default dartModule;
