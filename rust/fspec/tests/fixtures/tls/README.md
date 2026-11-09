Test-only TLS material for RPC-456 (rust/fspec/tests/rpc456_remote_daemon_over_tls.rs).

- `ca.pem`: a throwaway test CA (its key was deleted after signing).
- `server.pem` / `server.key`: a leaf for `localhost` and `127.0.0.1` signed by that CA.

Never use these outside tests.
