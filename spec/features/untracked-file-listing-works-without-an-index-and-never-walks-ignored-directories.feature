@done
@BUG-592
@git
@git-ops
@rust
@scalability
Feature: Untracked-file listing works without an index and never walks ignored directories
  """
  codelet-git's untracked-file listing (get_untracked_files, and the untracked
  leg of capture_changed_files) failed in a git working tree with no index
  file (a fresh git init with nothing staged), and it walked every directory,
  including ignored ones such as a build output tree, checking each file
  against the ignore rules only afterwards. Now a missing index reads as an
  empty one, and the walk never enters a directory the repository's ignore
  rules exclude. As in git, a file under an excluded directory cannot be
  re-included, so the listing is unchanged; only the walk is shorter. A
  directory the walk enters but cannot read is an error naming it, never
  silently skipped. Exact facts only and language-agnostic: the listing uses
  git's ignore rules, never a list of build-directory names. Core:
  rust/git/src/status.rs. Tests:
  rust/git/tests/bug592_untracked_listing_without_index_skips_ignored_dirs.rs.
  """

  Background: User Story
    As a developer running fspec audit or a git status capture
    I want to list the untracked files of any git working tree, including one with no index, without walking ignored directories
    So that the listing never fails on a fresh repository and costs time in proportion to the files git would show, not the build output it ignores

  Scenario: A fresh repository with no index lists every file the ignore rules do not exclude
    Given a fresh git repository with no index file
    And its .gitignore holds "build/" and it has the files a.txt and build/out.bin
    When its untracked files are listed
    Then the listing succeeds with exactly .gitignore and a.txt

  Scenario: The walk never enters an ignored directory
    Given a git repository with a commit whose .gitignore holds "target/"
    And an untracked file notes.txt and a directory target/ that cannot be read
    When its untracked files are listed
    Then the listing succeeds with exactly notes.txt

  Scenario: A directory that is not ignored and cannot be read is an error naming it
    Given a git repository with a commit and no ignore rule for notes/
    And a directory notes/ that cannot be read
    When its untracked files are listed
    Then the listing fails with an error that names notes
