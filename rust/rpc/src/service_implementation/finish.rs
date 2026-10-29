macro_rules! compose_rpc_finish {
    ($($collected:tt)*) => {
        impl FspecService for FspecServiceImpl {
            $($collected)*
        }
    };
}
