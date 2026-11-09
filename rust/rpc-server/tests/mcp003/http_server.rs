//! MCP-003/MCP-004/MCP-006 test support: the fixture's network servers.

use crate::support::{fixture, Boxed};

/// Start the fixture with `args` and return it with the port it announced.
pub fn fixture_server(args: &[&str]) -> Boxed<(std::process::Child, u16)> {
    use std::io::BufRead;
    let mut child = std::process::Command::new(fixture())
        .args(args)
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().ok_or("fixture stdout")?;
    let mut line = String::new();
    std::io::BufReader::new(stdout).read_line(&mut line)?;
    Ok((child, line.trim().parse()?))
}

/// Start the fixture's Streamable HTTP server and return it with its url.
/// With `bearer` (MCP-004) it refuses every request without that bearer.
pub fn http_fixture(bearer: Option<&str>) -> Boxed<(std::process::Child, String)> {
    let mut args = vec!["--http"];
    if let Some(token) = bearer {
        args.extend(["--bearer", token]);
    }
    let (child, port) = fixture_server(&args)?;
    Ok((child, format!("http://127.0.0.1:{port}/mcp")))
}
