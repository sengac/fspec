// REFAC-020: terminal stage emitting the complete original `Mode` enum.
macro_rules! compose_mode_finish {
    ($($collected:tt)*) => {
        #[derive(Subcommand, Debug)]
        enum Mode {
            $($collected)*
        }
    };
}
