macro_rules! compose_service_finish {
    ($($collected:tt)*) => {
        /// The fspec RPC service surface.
        ///
        /// All methods take a `tarpc::context::Context` (injected by the macro) and
        /// return owned values that implement `serde::Serialize + Deserialize`.
        #[tarpc::service]
        pub trait FspecService {
            $($collected)*
        }
    };
}
