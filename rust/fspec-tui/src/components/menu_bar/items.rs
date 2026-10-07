//! MENU-001 — the MenuCategories registry: single source of truth for the
//! menu bar's dropdown entries.
//!
//! Feature: spec/features/menubar-component-2-zone-bar-gui-menu-items-anchored-dropdown-with-view-chip-switcher-menucategories-registry-pure-painter.feature
//! Card: MENU-001 (R2, R8, R10).
//!
//! The registry is consumed by FOUR surfaces so they can never drift:
//!   - the MenuDropdown row builder (components/menu_bar/dropdown.rs)
//!   - the 'u' menu-bar help dialog (components/menu_bar_help.rs)
//!   - the bare board key arms' documentation (views/board/keys.rs)
//!   - the board help content (components/help_content.rs)
//!
//! MENU-008: the Zone A categories were reorganized into four groups in
//! fixed order (R2):
//!   - "Kanban":   . New Agent, / Search, D FOUNDATION.md, A Attachments
//!   - "Tools":    F Changed Files, C Checkpoints
//!   - "Settings": M Mux, P Providers
//!   - "Help":     ? Help, Esc Exit
//!
//! Each entry carries the key hint, label, one-line description, and the
//! `MenuAction` it resolves to. `MenuAction::to_action` maps onto the
//! EXISTING `Action` variants (no new bus tokens).
//!
//! R8: the `NewAgent` entry carries NO session payload in the registry —
//! the caller substitutes its live `Option<SessionId>` target via
//! `to_action` (BOARD-023 R5 snapshot semantics, but the registry stays
//! `&'static`).

use codelet_rpc_types::SessionId;

use crate::components::Action;

/// The action a dropdown row / help-dialog row executes. All variants map
/// onto existing `Action`s so `App::dispatch` needs no new arms in
/// MENU-001 (the ring-navigation actions land with MENU-002/003).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    /// `.` New Agent — `OpenAgentView(target)` where `target` is the
    /// caller's session snapshot at execute time (R8).
    NewAgent,
    /// `/` Search work units.
    Search,
    /// `A` Open the attachment picker for the selected work unit.
    Attachments,
    /// `C` Open the checkpoints view.
    Checkpoints,
    /// `F` Open the changed-files view.
    ChangedFiles,
    /// `D` Open FOUNDATION.md in the browser.
    Foundation,
    /// `M` Open the mux layout config.
    Mux,
    /// `P` Open the provider settings view (RPC-054). MENU-008.
    Providers,
    /// `?` Show the board keybinding help.
    Help,
    /// `Esc` Confirm exiting fspec.
    Exit,
    /// MENU-009 Zone C: the agent bar's `Close Agent [esc]` button —
    /// BUG-199: the button IS the Esc gesture, so it runs the
    /// AgentEscPressed cascade (running → interrupt, draft → clear,
    /// else the 'Exit Session?' ExitConfirmationDialog — RPC-098 L7)
    /// instead of the direct `AgentExitChoice{CloseSession}` teardown.
    CloseAgent,
}

impl MenuAction {
    /// Resolve to the concrete [`Action`] to emit. `new_agent_target`
    /// carries the session target snapshot for [`MenuAction::NewAgent`]
    /// (ignored by every other variant).
    pub fn to_action(self, new_agent_target: Option<SessionId>) -> Action {
        match self {
            MenuAction::NewAgent => Action::OpenAgentView(new_agent_target),
            MenuAction::Search => Action::OpenWorkUnitSearch,
            MenuAction::Attachments => Action::OpenAttachmentPicker,
            MenuAction::Checkpoints => Action::OpenCheckpointsView,
            MenuAction::ChangedFiles => Action::OpenChangedFilesView,
            MenuAction::Foundation => Action::OpenFoundation,
            MenuAction::Mux => Action::OpenMuxConfigDialog,
            MenuAction::Providers => Action::OpenProviderSettingsView,
            MenuAction::Help => Action::OpenBoardHelp,
            MenuAction::Exit => Action::OpenBoardExitConfirmation,
            // BUG-199 R2: the agent bar's 'Close Agent [esc]' button is
            // the Esc gesture — the same cascade `AgentEscPressed` runs
            // (interrupt / draft-clear / ExitConfirmationDialog).
            MenuAction::CloseAgent => Action::AgentEscPressed,
        }
    }
}

/// One dropdown / help-dialog row: key hint, label, one-line description,
/// and the action it executes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuEntry {
    /// The key hint painted in the dropdown's 4-cell left column
    /// (e.g. `"."`, `"/"`, `"Esc"`).
    pub key: &'static str,
    /// The label shown next to the key (e.g. `"New Agent"`).
    pub label: &'static str,
    /// The one-line description (dimmed in the dropdown).
    pub description: &'static str,
    /// The action the row executes.
    pub action: MenuAction,
}

/// One top-level menu item (Zone A) with its dropdown entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuCategory {
    /// Stable slug (documentary — tests address categories by index or
    /// label, so the slug is free to evolve).
    pub id: &'static str,
    /// The label painted as the Zone A item (e.g. `"Kanban"`).
    pub label: &'static str,
    /// The dropdown entries in display order.
    pub entries: &'static [MenuEntry],
}

/// MENU-008 R2: the 4 Kanban (work-unit workflow) entries — the legacy
/// `.`/`/`/`D`/`A` BOARD-023 arms.
pub const KANBAN: &[MenuEntry] = &[
    MenuEntry {
        key: ".",
        label: "New Agent",
        description: "Open/start an agent for the focused unit",
        action: MenuAction::NewAgent,
    },
    MenuEntry {
        key: "/",
        label: "Search",
        description: "Search work units",
        action: MenuAction::Search,
    },
    MenuEntry {
        key: "D",
        label: "FOUNDATION.md",
        description: "Open FOUNDATION.md in the browser",
        action: MenuAction::Foundation,
    },
    MenuEntry {
        key: "A",
        label: "Attachments",
        description: "Open the attachment picker for the selected unit",
        action: MenuAction::Attachments,
    },
];

/// MENU-008 R3: the 2 git-tool entries.
pub const TOOLS: &[MenuEntry] = &[
    MenuEntry {
        key: "F",
        label: "Changed Files",
        description: "Open the changed-files view",
        action: MenuAction::ChangedFiles,
    },
    MenuEntry {
        key: "C",
        label: "Checkpoints",
        description: "Open the checkpoints view",
        action: MenuAction::Checkpoints,
    },
];

/// MENU-008 R4: the 2 configuration entries.
pub const SETTINGS: &[MenuEntry] = &[
    MenuEntry {
        key: "M",
        label: "Mux",
        description: "Open the mux layout config",
        action: MenuAction::Mux,
    },
    MenuEntry {
        key: "P",
        label: "Providers",
        description: "Open the provider settings view",
        action: MenuAction::Providers,
    },
];

/// The 2 Help entries (unchanged — MENU-008 R5).
pub const HELP: &[MenuEntry] = &[
    MenuEntry {
        key: "?",
        label: "Help",
        description: "Show the full menu bar help",
        action: MenuAction::Help,
    },
    MenuEntry {
        key: "Esc",
        label: "Exit",
        description: "Confirm exiting fspec",
        action: MenuAction::Exit,
    },
];

/// MENU-008 R1: the Zone A category list in fixed order — `Kanban`,
/// `Tools`, `Settings`, then `Help`.
pub const CATEGORIES: &[MenuCategory] = &[
    MenuCategory {
        id: "kanban",
        label: "Kanban",
        entries: KANBAN,
    },
    MenuCategory {
        id: "tools",
        label: "Tools",
        entries: TOOLS,
    },
    MenuCategory {
        id: "settings",
        label: "Settings",
        entries: SETTINGS,
    },
    MenuCategory {
        id: "help",
        label: "Help",
        entries: HELP,
    },
];
