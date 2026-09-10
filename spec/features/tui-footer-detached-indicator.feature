@done
@bug-fix
@wt-002
@tui
@session
@critical
@component
@feature-group
Feature: TUI Footer Detached Indicator
  """
  TUI FooterStateUpdate dispatch (rust/fspec-tui/src/app/dispatch_stream_chunks.rs): is_git_repo=false collapses git_branch to None (blank, no repo); is_git_repo=true with branch=None is a detached-HEAD worktree (fspec worktrees are created detached) and must surface a visible "(detached)" indicator in the footer right line, so the footer agrees with the isolation badge. Part of WT-002 (the fspec binary footer poller) — the poller emits is_git_repo=true/branch=None and this dispatch derives the indicator from the same chunk.
  """

  Background: User Story
    As a developer running the fspec TUI
    I want the session footer to show a visible detached-HEAD indicator for isolated worktree sessions
    So that the footer agrees with the isolation badge instead of rendering a blank branch

  Scenario: The footer renders a detached indicator for a detached-HEAD worktree instead of a blank branch
    Given an App with an open session s-1
    When the chunks subscriber forwards Action::ChunkReceived(s-1, StreamChunk::FooterStateUpdate { cwd: "<worktree>", display_path: "<worktree>", is_git_repo: true, branch: None })
    Then the stored workspace info carries the worktree CWD
    And the footer shows the worktree path with a detached indicator rather than a blank branch

  Scenario: The footer still collapses to a blank branch for a CWD that is not a git repository
    Given an App with an open session s-1
    When the chunks subscriber forwards Action::ChunkReceived(s-1, StreamChunk::FooterStateUpdate { cwd: "/tmp/not-a-repo", display_path: "/tmp/not-a-repo", is_git_repo: false, branch: None })
    Then the stored workspace info carries the CWD
    And the footer shows no branch indicator for a non-repo CWD
