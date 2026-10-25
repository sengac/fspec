@done
@BUG-266
@providers
@completion
Feature: Custom Done settlement
  """
  The real custom rig adapter settles physical usage at an authoritative
  transport boundary before releasing native Done. Normal attribution and
  admission remain active; the fixture uses synthetic local HTTP only.
  """

  Background: User Story
    As a user of a scripted provider
    I want completion to preserve trailing physical usage
    So that native Done cannot conceal billed inference

  Scenario: Native Done cannot discard trailing physical usage
    Given a local custom stream queues an attributed armed native Done before stop and trailing usage
    When normal admission and the shared stream loop consume the response
    Then exact physical usage settles once before accepted Done without a nudge
