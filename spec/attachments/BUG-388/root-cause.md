# BUG-388 root cause

`refac025_mock_split::every_fspec_tui_test_helper_file_stays_under_300_lines` fails at HEAD (e351762be):
`rust/fspec-tui/tests/common/mock_default.rs` is 300 lines and the REFAC-025 ceiling is `< 300`.

BUG-379 (8bf7f9372) added one field to `MockBackend`'s `Default` impl, which took the file from 299 to 300 lines.
The impl spends 19 lines opening the nine broadcast channels (`let (x_tx, _) = broadcast::channel(64);` then
`x_tx: Mutex::new(Some(x_tx))`), so every new channel or field pushes the file toward the ceiling.

Fix: an `open_channel()` helper builds each `Mutex<Option<broadcast::Sender<T>>>` slot in place, removing the nine
`let` lines and taking the file to 296 lines without moving fields.
