//! Feature: spec/features/board-card-enter-chooser-and-complete-details.feature

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{SessionId, WorkUnitContext};

#[path = "board024/attachments.rs"]
mod attachments;
#[path = "board024/browser.rs"]
mod browser;
#[path = "board024/canonical.rs"]
mod canonical;
#[path = "board024/chooser.rs"]
mod chooser;
mod common;
#[path = "board024/details.rs"]
mod details;
#[path = "session_intent/fixture.rs"]
mod intent_fixture;
#[path = "board024/loading.rs"]
mod loading;
#[path = "board024/selection.rs"]
mod selection;
#[path = "board024/support.rs"]
mod support;

use canonical::Canonical;
use intent_fixture::{Isolation, TestResult};
use support::*;
