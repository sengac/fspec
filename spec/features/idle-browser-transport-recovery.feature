@done
@phase-1
@browser
@web-tools
@error-handling
@BUG-237
Feature: Idle browser transport recovery
  """
  Installed-Chrome behavioral coverage reproduces idle expiry with a controlled
  one-second transport timeout. The cache-policy contract lives separately in
  cached-browser-recovery.feature so deterministic tests require no browser.
  """

  Scenario: Recover an idle expired launched browser
    Given a cached launched browser whose idle transport has expired
    When the next operation requests a browser
    Then a healthy replacement is returned with the same configuration

  Scenario: Recover an idle expired connected browser
    Given a cached connection to an externally owned browser whose idle transport has expired
    When the next operation requests a browser
    Then a healthy connection is returned without stopping the external browser
