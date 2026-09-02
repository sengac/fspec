#!/usr/bin/env python3
"""IPv4-only HTTP CONNECT forward proxy bound to 127.0.0.1.

Why this exists
---------------
Cross-compiling Windows binaries uses `cargo xwin` (cargo-xwin), which downloads
the MSVC CRT / SDK payloads from Microsoft's CDN (`download.visualstudio.microsoft.com`)
via the `ureq` HTTP client. On hosts whose **IPv6 egress is broken but whose IPv4
egress works**, Microsoft's CDN answers DNS with an **AAAA (IPv6) record that
`getaddrinfo` orders first** (RFC 8305). `ureq`/Rust's `TcpStream::connect` tries
that IPv6 address, gets `EHOSTUNREACH` ("No route to host"), and — because Rust
only falls back to the next address on `ECONNABORTED`, **not** on `EHOSTUNREACH`
— it never tries the working IPv4 address. The build then dies with:

    Error: Failed to setup MSVC CRT
      Caused by: HTTP GET request for https://download.visualstudio.microsoft.com/...
                 failed; io: No route to host (os error 113)

`ureq` (and most HTTP clients) honour the `HTTPS_PROXY` / `https_proxy`
environment variables. Pointing them at this tiny local CONNECT proxy that
**resolves targets with `AF_INET` only** (A records, never AAAA) forces the whole
download onto IPv4. It is safe: the proxy binds to loopback only, and it merely
forwards TCP bytes (TLS is still done end-to-end by the client).

Usage
-----
    python3 ipv4-proxy.py [port]

If `port` is omitted or `0`, the OS picks a free port. The process prints the
bound port on line 1 and `READY` on line 2, then serves until killed.
"""

import os
import select
import socket
import sys
import threading


def _resolve_ipv4(host: str, port: int):
    """Resolve `host` to an IPv4 address only (AF_INET). Never returns IPv6."""
    infos = socket.getaddrinfo(host, port, socket.AF_INET, socket.SOCK_STREAM)
    if not infos:
        raise OSError("no IPv4 address for %s" % host)
    return infos[0][4]


def _relay(a, b):
    """Copy bytes in both directions until either side closes."""
    try:
        while True:
            readable, _, _ = select.select([a, b], [], [], 300)
            if not readable:
                return
            for sock in readable:
                data = sock.recv(65536)
                if not data:
                    return
                (b if sock is a else a).sendall(data)
    except OSError:
        pass
    finally:
        for sock in (a, b):
            try:
                sock.close()
            except OSError:
                pass


def _handle(client):
    try:
        req = b""
        while b"\r\n\r\n" not in req:
            chunk = client.recv(4096)
            if not chunk:
                return
            req += chunk
        line = req.split(b"\r\n", 1)[0].decode("latin-1")
        parts = line.split()
        if len(parts) < 2 or parts[0] != "CONNECT":
            client.sendall(b"HTTP/1.1 400 Bad Request\r\n\r\n")
            return
        target = parts[1]
        host, _, port_str = target.partition(":")
        port = int(port_str) if port_str else 443
        print("CONNECT %s -> %s:%d" % (target, host, port), file=sys.stderr, flush=True)
        try:
            server = socket.create_connection(_resolve_ipv4(host, port), timeout=15)
        except OSError as exc:
            body = "IPv4-proxy: %s\r\n" % exc
            client.sendall(("HTTP/1.1 502 Bad Gateway\r\nContent-Length: %d\r\n\r\n%s" % (len(body), body)).encode())
            return
        client.sendall(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        _relay(client, server)
    except OSError:
        pass
    finally:
        try:
            client.close()
        except OSError:
            pass


def main():
    port = 0
    if len(sys.argv) > 1:
        try:
            port = int(sys.argv[1])
        except ValueError:
            port = 0
    srv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    srv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    srv.bind(("127.0.0.1", port))
    srv.listen(128)
    # Line 1: the bound port (useful when 0 was requested). Line 2: READY.
    print(srv.getsockname()[1], flush=True)
    print("READY", flush=True)
    while True:
        conn, _ = srv.accept()
        threading.Thread(target=_handle, args=(conn,), daemon=True).start()


if __name__ == "__main__":
    main()
