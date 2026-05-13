@done
@rust
@tarpc
@rpc
@websocket
@envelope
@regression
@RPC-007
Feature: Reserved Envelope variants narrowed after RPC-007
  """
  RPC-005 reserved-and-rejected the Envelope variants Event, LogEvent,
  CmdReq, and CmdRes (only Rpc was legitimate). RPC-006 narrowed the
  rejected list to {Event, LogEvent, CmdReq, CmdRes} when WorkUnitsUpdate
  became legitimate. RPC-007 narrows it further to {CmdReq, CmdRes}: Event
  and LogEvent are now legitimate variants. CmdReq and CmdRes remain
  reserved-and-rejected — the server still counts them in
  ServerStats.rejected_envelopes and ServerStats.rejected_variants.
  Reverse callbacks (callFspecCommand) are explicitly out of scope per
  RPC-002 §8.
  """

  Background: User Story
    As a developer maintaining the WebSocket envelope protocol
    I want the server to keep rejecting CmdReq and CmdRes while accepting Event and LogEvent
    So that future cards can light up reverse callbacks without ambiguity, and current rejections continue to be observable through ServerStats

  @rpc
  @websocket
  @envelope
  @regression
  Scenario: Envelope::CmdReq and Envelope::CmdRes remain reserved-and-rejected while Event and LogEvent are now legitimate
    Given a WebSocket client is connected to codelet-rpc-server
    When the client sends an Envelope::CmdReq frame
    Then the server rejects the frame and increments ServerStats.rejected_envelopes
    And ServerStats.rejected_variants includes "CmdReq"
    When the client sends an Envelope::CmdRes frame
    Then the server rejects the frame and ServerStats.rejected_variants includes "CmdRes"
    And ServerStats.rejected_variants does NOT include "Event" or "LogEvent"
