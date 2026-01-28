// Simple test to prove NAPI callback pattern works from TypeScript
// This calls the actual codelet NAPI module with a real callback

import { testCallback } from '@sengac/codelet-napi';

console.log('🧪 Testing NAPI callback pattern...');

try {
  // Call the NAPI function with a real callback
  const result = testCallback(
    'Hello from fspec TypeScript',
    (input: string) => {
      console.log(`📥 Callback received: ${input}`);
      // This is what the callback does - transform the input
      return `✅ Processed: ${input}`;
    }
  );

  console.log(`📤 Final result: ${result}`);

  if (result === '✅ Processed: Hello from fspec TypeScript') {
    console.log('🎉 SUCCESS: NAPI callback pattern works!');
    console.log('✅ Rust called TypeScript callback successfully');
    console.log('✅ TypeScript callback executed and returned result');
    console.log('✅ Result flowed back to Rust and then to TypeScript');
  } else {
    console.log(
      `❌ FAILED: Expected "✅ Processed: Hello from fspec TypeScript", got "${result}"`
    );
    process.exit(1);
  }
} catch (error) {
  console.error('❌ ERROR:', error);
  process.exit(1);
}

console.log('\n🎯 This proves the NAPI callback pattern works correctly!');
console.log(
  '📋 Flow: fspec TypeScript → codelet Rust NAPI → TypeScript callback → Result back to fspec TypeScript'
);
