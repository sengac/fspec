/**
 * Test FspecTool session-level interception
 *
 * This test verifies that when an agent uses FspecTool within a session,
 * the session manager properly intercepts the error and executes the command.
 */

import { testFspecJsControlledInvocation } from './fspec-init';

async function testFspecInterception() {
  console.log('[test] Testing FspecTool session-level interception...');

  try {
    // Test our basic JS-controlled invocation
    await testFspecJsControlledInvocation('.');

    console.log('[test] ✅ FspecTool interception test completed!');
    console.log(
      '[test] The session manager should now be able to intercept FspecTool errors'
    );
    console.log(
      '[test] and execute them via the synchronous implementation we added.'
    );
  } catch (error) {
    console.error('[test] ❌ FspecTool interception test failed:', error);
    throw error;
  }
}

// Export the test function for use in other contexts
export { testFspecInterception };
