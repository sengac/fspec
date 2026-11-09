# REFAC-061 design

Move DATA_DIR_GUARD, setup_data_dir and OFFLINE_DEFAULT_MODEL to tests/rpc002_session_persistence_support/mod.rs. Write a feature for the RPC-002 round-trip scenarios (full round trip, token state, message ordering, unknown session, independent sessions), split across two features if the tests stay over 300 lines, and link coverage.
