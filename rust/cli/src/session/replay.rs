#[derive(Debug, Clone)]
pub struct ReplayInstallation {
    pub revision: u64,
    pub runtime_prefix_len: usize,
    pub replay_len: usize,
}
