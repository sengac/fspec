// REFAC-020: terminal stage emitting the complete original dispatch match.
macro_rules! route_dispatch_finish {
    ($command:expr, $workspace:expr, $forward:ident; $($collected:tt)*) => {
        match $command { $($collected)* }
    };
}
