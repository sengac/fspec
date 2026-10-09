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
//! EXISTING `Action` variants (no new bus tokens) — payload-free: the
//! `NewAgent` entry's MENU-009 R8 session-target substitution was removed
//! by BUG-199 (agent surface) and BUG-203 (board surface) — 'New Agent'
//! ALWAYS mounts the CreateSessionDialog (the shared RPC-060 helper), it
//! never re-enters / resumes a session (that is the Shift+Right CYCLE
//! gesture's `Action::OpenAgentView` job).

use crate::components::Action;

/// The action a dropdown row / help-dialog row executes. All variants map
/// onto existing `Action`s so `App::dispatch` needs no new arms in
/// MENU-001 (the ring-navigation actions land with MENU-002/003).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    /// `.` New Agent — BUG-199 / BUG-203: ALWAYS start a NEW agent. The
    /// row mounts the CreateSessionDialog via the shared RPC-060 helper
    /// (`Action::OpenCreateSessionDialog { preselect: None }`, the
    /// agent-bar button's `dispatch_menu_zone_c` arm — exact parity).
    /// The MENU-009 R8 current-session / selected-unit target
    /// substitution is REMOVED: 'New Agent' must never re-enter /
    /// resume an existing session (that is the Shift+Right CYCLE
    /// gesture's job, `Action::OpenAgentView`). The mapping is
    /// payload-free — `to_action` takes no target.
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
    /// MENU-009 Zone C: the agent bar's `Close Agent` button —
    /// BUG-204 (superseding BUG-199 R2): the button ALWAYS mounts the
    /// 'Exit Session?' ExitConfirmationDialog (RPC-098 L7) when a
    /// session is open (no interrupt / no draft-clear branch — those
    /// belong to the physical Esc key only). The mapping below is a
    /// payload-free board-side fallback: the agent surface resolves
    /// `CloseAgent` through its OWN dispatch arm (`dispatch_menu_zone_c`),
    /// and no board Zone C carries `CloseAgent`, so this arm is
    /// unreachable in production.
    CloseAgent,
}

impl MenuAction {
    /// Resolve to the concrete [`Action`] to emit. Every variant maps to
    /// a payload-free `Action` — BUG-199 / BUG-203 removed the
    /// `NewAgent` session-target substitution (the 'New Agent' row /
    /// button ALWAYS mounts the CreateSessionDialog via the shared
    /// RPC-060 helper; it never re-enters or resumes a session).
    pub fn to_action(self) -> Action {
        match self {
            // BUG-199 / BUG-203: 'New Agent' ALWAYS starts a new agent —
            // the shared RPC-060 helper (the agent-bar button's
            // `dispatch_menu_zone_c` arm — exact parity, no payload).
            MenuAction::NewAgent => Action::OpenCreateSessionDialog { preselect: None },
            MenuAction::Search => Action::OpenWorkUnitSearch,
            MenuAction::Attachments => Action::OpenAttachmentPicker,
            MenuAction::Checkpoints => Action::OpenCheckpointsView,
            MenuAction::ChangedFiles => Action::OpenChangedFilesView,
            MenuAction::Foundation => Action::OpenFoundation,
            MenuAction::Mux => Action::OpenMuxConfigDialog,
            MenuAction::Providers => Action::OpenProviderSettingsView,
            MenuAction::Help => Action::OpenBoardHelp,
            MenuAction::Exit => Action::OpenBoardExitConfirmation,
            // BUG-204 (superseding BUG-199 R2): the agent bar's
            // 'Close Agent' button ALWAYS mounts the 'Exit Session?'
            // dialog — the agent surface resolves `CloseAgent` through
            // its OWN dispatch arm (`dispatch_menu_zone_c`), so this
            // mapping is a payload-free board-side fallback (no board
            // Zone C carries `CloseAgent` — unreachable in production).
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

impl MenuCategory {
    /// MENU-011 R1: the Zone A item's bracketed display label —
    /// `"Kanban"` → `"[ Kanban ]"` (a space between the word and each
    /// bracket). The brackets are part of the button: the layout pass,
    /// the painter and the hit-test rects all cover them. The registry
    /// `label` itself stays PLAIN — the dropdown rows and the 'u' help
    /// dialog keep their plain labels (bracketing is a bar-row-only
    /// paint-time transform).
    pub fn bracketed_label(&self) -> String {
        format!("[ {} ]", self.label)
    }
}

/// MENU-008 R2: the 4 Kanban (work-unit workflow) entries — the legacy
/// `.`/`/`/`D`/`A` BOARD-023 arms.
pub const KANBAN: &[MenuEntry] = &[
    MenuEntry {
        key: ".",
        label: "New Agent",
        description: "Always start a new agent (never resumes)",
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
