//! Feature: spec/features/session-creation-intent.feature

use std::sync::Arc;

use codelet_fspec_tui::{Action, App, FspecBackend, ViewMode};
use codelet_rpc_types::{SessionId, WorkUnitContext, WorkUnitFocus};

#[path = "session_intent/board.rs"]
mod board;
mod common;
#[path = "session_intent/creation.rs"]
mod creation;
#[path = "session_intent/fixture.rs"]
mod fixture;
#[path = "session_intent/generic.rs"]
mod generic;

use fixture::*;
