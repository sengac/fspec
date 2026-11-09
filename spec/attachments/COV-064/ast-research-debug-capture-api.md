rust/common/src/debug_capture/mutex.rs:8:5:pub fn new(value: T) -> Self {
rust/common/src/debug_capture/mutex.rs:15:5:pub fn lock(&self) -> Result<std::sync::MutexGuard<'_, T>, DebugCaptureError> {
rust/common/src/debug_capture/capture.rs:16:5:pub fn capture(&mut self, event_type: &str, data: Value, options: Option<CaptureOptions>) {
rust/common/src/debug_capture/session_lifecycle.rs:15:5:pub(super) fn update_latest_symlink(&self) {
rust/common/src/debug_capture/session_lifecycle.rs:34:5:pub(super) fn capture_session_start(&mut self) {
rust/common/src/debug_capture/session_lifecycle.rs:58:5:pub(super) fn capture_session_end(&mut self) {
rust/common/src/debug_capture/mod.rs:34:1:pub fn get_debug_capture_manager(
rust/common/src/debug_capture/mod.rs:47:1:pub fn handle_debug_command() -> DebugCommandResult {
rust/common/src/debug_capture/mod.rs:60:1:pub fn capture_event(event_type: &str, data: serde_json::Value) {
rust/common/src/debug_capture/mod.rs:76:1:pub fn increment_debug_turn() {
rust/common/src/debug_capture/mod.rs:89:1:pub fn handle_debug_command_with_dir(base_dir: Option<&str>) -> DebugCommandResult {
rust/common/src/debug_capture/summary.rs:15:5:pub fn format_duration(&self, ms: u64) -> String {
rust/common/src/debug_capture/summary.rs:30:5:pub(super) fn summarize_event_data(&self, event: &DebugEvent) -> String {
rust/common/src/debug_capture/summary.rs:88:5:pub(super) fn generate_summary(&self, session_file: &str) {
rust/common/src/debug_capture/manager.rs:20:1:pub(super) fn sanitize_headers(headers: &Value) -> Value {
rust/common/src/debug_capture/manager.rs:64:5:pub fn new() -> Result<Self, DebugCaptureError> {
rust/common/src/debug_capture/manager.rs:91:5:pub fn set_debug_directory(&mut self, base_dir: PathBuf) {
rust/common/src/debug_capture/manager.rs:100:5:pub fn set_debug_directory_raw(&mut self, dir: PathBuf) {
rust/common/src/debug_capture/manager.rs:105:5:pub fn is_enabled(&self) -> bool {
rust/common/src/debug_capture/manager.rs:115:5:pub fn set_session_metadata(&mut self, metadata: SessionMetadata) {
rust/common/src/debug_capture/manager.rs:137:5:pub fn start_capture(&mut self) -> Result<String, DebugCaptureError> {
rust/common/src/debug_capture/manager.rs:195:5:pub fn stop_capture(&mut self) -> Result<String, DebugCaptureError> {
rust/common/src/debug_capture/manager.rs:235:5:pub fn increment_turn(&mut self) {
rust/common/src/debug_capture/manager.rs:240:5:pub(super) fn get_session_duration(&self) -> u64 {

rust/common/src/logging/private_files.rs:4:1:pub fn appender(directory: &Path, prefix: &str) -> std::io::Result<RollingFileAppender> {
rust/common/src/logging/mod.rs:77:1:pub fn init_logging(verbose: bool) -> Result<()> {
rust/common/src/logging/privacy_json.rs:3:1:pub(super) fn sanitize(text: &str, payloads: bool) -> String {
rust/common/src/logging/privacy.rs:58:5:pub fn local() -> Self {
rust/common/src/logging/privacy.rs:119:1:pub fn redact(value: &str) -> String {
rust/common/src/logging/privacy.rs:148:5:pub fn new(policy: PrivacyFields) -> Self {
rust/common/src/logging/privacy.rs:180:5:pub fn update(&mut self, fields: impl RecordFields) {
rust/common/src/logging/privacy.rs:184:5:pub fn render(&self) -> String {
rust/common/src/logging/privacy.rs:226:1:pub fn event_fields(event: &Event<'_>, policy: PrivacyFields) -> DiagnosticFields {

