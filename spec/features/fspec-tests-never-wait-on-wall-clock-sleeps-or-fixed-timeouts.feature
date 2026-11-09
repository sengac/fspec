@wip
@testing
@rust
@test-infrastructure
@validator
@TEST-042
Feature: fspec tests never wait on wall-clock sleeps or fixed timeouts
  """
  New rust/test-helpers/src/deadlock_guard.rs: DEADLOCK_GUARD = 300 s; deadlock_guard(what, future) -> impl Future<Output = Result<T, Failure>> (a watcher thread waits on a channel with recv_timeout and wakes the task; dropping the guard ends the thread) and deadlock_guard_blocking(what, closure) -> Result<T, Failure> (the closure runs on its own thread, the caller waits with recv_timeout). New rust/test-helpers/src/wall_clock_waits.rs: the exact scan (proc-macro2 token walk of each file with span lines, use-tree aliases scoped to the enclosing brace group, syn for #[cfg(test)] / #[test] item line ranges in src), returning file:line: forbidden-path offences. New test rust/test-helpers/tests/test042_no_wall_clock_waits_in_tests.rs: scan unit scenarios on fixture sources plus the whole-workspace scan that must report nothing.
  """

  # ========================================
  # EXAMPLE MAPPING CONTEXT
  # ========================================
  #
  # BUSINESS RULES:
  #   1. Test code is every workspace member's tests/ tree, plus the #[cfg(test)] items and #[test] / #[tokio::test] functions of its src files; rust/patches (upstream rig-core, not a workspace member) is not fspec's own suite and is not scanned
  #   2. Forbidden in test code (exact paths, resolved through the file's use declarations, including inside macros): std::thread::sleep, tokio::time::sleep, sleep_until, timeout, timeout_at, interval and interval_at, and the timed-wait methods recv_timeout, wait_timeout, wait_timeout_while and wait_timeout_ms
  #   3. A test waits on the event or state it asserts (a channel message, a notification, a joined task, a state read after the awaited operation); a test that asserts something does not happen asserts it at a point where it would already have happened (after the operation that would cause it has completed), never by waiting a while
  #   4. The only timed wait is a generous deadlock guard through one named helper, codelet_test_helpers::deadlock_guard (async, runtime-agnostic) and deadlock_guard_blocking (a closure on its own thread): it returns an error naming what was awaited and the guard duration when it expires, so the test fails loudly; its own implementation in rust/test-helpers/src/deadlock_guard.rs is the one place a timed wait exists
  #   5. test-helpers stays free of tokio (RPC-067, BUG-487), so the guard uses a std thread and a channel, never tokio::time
  #   6. Paused-clock tests are not wall-clock waits: inside a function annotated #[tokio::test(start_paused = true)] the tokio sleep, timeout and interval calls run on virtual time and are not reported (exact attribute fact). This replaces the earlier choice to forbid them there too: the card forbids wall-clock waits, and tests of timeout composition (stall timeouts, operation deadlines) need virtual time
  #   7. No allowlist: the check reports every offender by file, line and forbidden call, and passes only when there are none; offenders are fixed in this card
  #   8. Clock comparisons are fixed deadlines too (supervisor, 2026-11-07): in test code a comparison (<, <=, >, >=) with an operand that reads the clock is reported: an .elapsed() call (also through a postfix chain such as .elapsed().as_millis()), std::time::Instant::now() or tokio::time::Instant::now() (resolved through use), or a local bound by let to an expression that reads the clock. Equality checks and clock readings that are never compared (printed or recorded) are not reported. Inside a paused-clock test only readings of std::time::Instant are reported, since tokio's clock is virtual there
  #   9. A test module's use super::* and super:: paths resolve through its parent module's use declarations (the parent of dir/x.rs is dir.rs or dir/mod.rs; of a top-level src file, lib.rs or main.rs), so a support module that reaches tokio::time::timeout through its parent's import is reported
  #   10. A file pulled in with include!("path") is part of the file that includes it: its names resolve through the including file's use declarations (the path is relative to the including file's directory), so an included part that calls timeout imported by its includer is reported
  #
  # EXAMPLES:
  #   1. A tests/ file calls tokio::time::sleep(Duration::from_millis(50)).await before reading a state: the check fails naming the file, the line and tokio::time::sleep
  #   2. A file has use tokio::time::timeout; and calls timeout(Duration::from_secs(5), rx.recv()) inside assert!: the check names tokio::time::timeout at that line
  #   3. A #[cfg(test)] module in src calls std::thread::sleep; the same call in non-test production code of that file is not reported
  #   4. A test defines its own fn timeout(...) and calls it, with no tokio::time import: nothing is reported
  #   5. deadlock_guard("the reply", rx.recv()).await? returns the reply when it arrives, and returns an error naming the reply and the guard duration when the future never completes
  #   6. A test calls rx.recv_timeout(Duration::from_secs(1)) on a std channel: the check names recv_timeout
  #   7. A #[tokio::test(start_paused = true)] function calls tokio::time::timeout and tokio::time::sleep: nothing is reported; the same calls in a plain #[tokio::test] are reported
  #   8. let started = Instant::now(); run(); assert!(started.elapsed() < Duration::from_millis(200)); is reported as elapsed() comparison, while Instant::now() < deadline as std::time::Instant::now() comparison, and let waited = started.elapsed(); assert!(waited >= limit) at the assert line; assert_eq!(clock.elapsed(), Duration::from_secs(1)) and println!("{:?}", started.elapsed()) are not reported
  #   9. tests/x/support.rs has use super::*; and calls timeout(limit, rx.recv()), and tests/x.rs has use tokio::time::timeout;: the check names tokio::time::timeout in support.rs
  #   10. In a #[tokio::test(start_paused = true)] function, std::thread::sleep and a std::time::Instant::now() comparison are reported; a tokio::time::Instant::now() comparison is not
  #   11. tests/x.rs has use tokio::time::timeout; and include!("x/part.rs"); tests/x/part.rs calls timeout(limit, rx.recv()) with no use of its own: the check names tokio::time::timeout in part.rs
  #
  # ========================================
  Background: User Story
    As a fspec maintainer
    I want to have a source check fail any fspec test that waits on a wall-clock sleep or a fixed timeout
    So that the suite passes or fails on what the code does, not on how loaded the machine is

  Scenario: A tokio sleep in a test file is reported
    Given a tests/ file that calls tokio::time::sleep before reading a state
    When the wall-clock wait check scans it
    Then it reports the file, the line and "tokio::time::sleep"

  Scenario: An imported timeout inside a macro is reported
    Given a tests/ file with "use tokio::time::timeout;" that calls timeout inside assert!
    When the wall-clock wait check scans it
    Then it reports that line and "tokio::time::timeout"

  Scenario: Only the test code of a src file is scanned
    Given a src file that calls std::thread::sleep in production code and in a #[cfg(test)] module
    When the wall-clock wait check scans it
    Then it reports only the call in the #[cfg(test)] module

  Scenario: A test's own function named timeout is not reported
    Given a tests/ file that defines and calls its own fn timeout without importing tokio::time
    When the wall-clock wait check scans it
    Then it reports nothing

  Scenario: Timed-wait methods are reported
    Given a tests/ file that calls recv_timeout on a std channel and wait_timeout on a condvar
    When the wall-clock wait check scans it
    Then it reports "recv_timeout" and "wait_timeout" with their lines

  Scenario: A paused-clock test may use tokio time
    Given a tests/ file whose #[tokio::test(start_paused = true)] function calls tokio::time::timeout and tokio::time::sleep
    And a plain #[tokio::test] function in the same file that calls tokio::time::sleep
    When the wall-clock wait check scans it
    Then it reports only the call in the plain test

  Scenario: The deadlock guard returns the awaited value or fails loudly
    Given a future that completes and a future that never completes
    When each is awaited through deadlock_guard with a short guard
    Then the completed future's value is returned
    And the never-completing one fails naming what was awaited and the guard duration

  Scenario: fspec's own test code waits on no wall-clock sleep or fixed timeout
    Given every workspace member's tests/ tree and the test code of its src files
    When the wall-clock wait check scans them
    Then it reports nothing

  Scenario: A comparison with a clock reading is reported
    Given a tests/ file that compares started.elapsed(), Instant::now() and a let-bound elapsed reading with a bound
    And the same file asserts an elapsed() equality and prints an elapsed() reading
    When the wall-clock wait check scans it
    Then it reports each comparison with its line and the clock it reads
    And it reports neither the equality nor the printed reading

  Scenario: A support module's glob import from its parent is resolved
    Given a tests/x.rs file with "use tokio::time::timeout;"
    And a tests/x/support.rs file with "use super::*;" that calls timeout
    When the wall-clock wait check scans the crate
    Then it reports "tokio::time::timeout" in tests/x/support.rs

  Scenario: A paused-clock test still may not read or sleep on the wall clock
    Given a #[tokio::test(start_paused = true)] function that calls std::thread::sleep
    And compares a std::time::Instant::now() reading and a tokio::time::Instant::now() reading with a bound
    When the wall-clock wait check scans it
    Then it reports "std::thread::sleep" and the std::time::Instant::now() comparison
    And it does not report the tokio::time::Instant::now() comparison

  Scenario: An included part resolves names through the file that includes it
    Given a tests/x.rs file with "use tokio::time::timeout;" that includes x/part.rs with include!
    And a tests/x/part.rs file with no use declarations that calls timeout
    When the wall-clock wait check scans the crate
    Then it reports "tokio::time::timeout" in tests/x/part.rs
