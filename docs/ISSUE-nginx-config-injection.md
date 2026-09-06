# [HIGH] Nginx config injection via unvalidated proxy route fields (SSRF / host file disclosure / arbitrary directives)

## Summary

`PUT /agents/{agent_id}/proxy-routes/{name}` accepts `server_name`, `path_prefix`, and `upstream_host` from the request body **with no validation**. These values are string-interpolated directly into an Nginx config file by the agent (`crates/agent/src/proxy.rs:28-31`). A user who can create proxy routes can inject arbitrary Nginx directives into the generated config — including a `location` block that reads host files (`alias /etc/`) or proxies to internal services (SSRF). The existing `nginx -t` check does not catch this, because a successfully injected config is syntactically valid.

## Impact

- **Host file disclosure** — inject a `location { alias /etc/; }` block and read arbitrary files from the machine running the agent (secrets, configs, credentials).
- **SSRF** — point `proxy_pass` at internal-only services (databases, admin panels, cloud metadata endpoints) that were never meant to be public.
- **Arbitrary Nginx configuration** — add any syntactically valid directives (headers, rewrites, new upstreams), effectively giving a scoped dashboard user full control over the reverse proxy.

Trust model: the route creator is not supposed to touch Nginx internals or reach the host filesystem; this lets them do both through what should be a simple "set this host/port" operation.

## Affected code

| Location | What's wrong |
|---|---|
| `crates/control-plane/src/http.rs:649-690` (`put_proxy_route`) | No validation of `server_name`, `path_prefix`, or `upstream_host` before storing |
| `crates/agent/src/proxy.rs:21-39` (`render_route`) | Values interpolated verbatim into the Nginx config via `writeln!` |
| `crates/agent/src/proxy.rs:92-102` (`apply`) | `nginx -t` only catches **syntactically invalid** config, not valid-but-malicious config |

The vulnerable interpolation:

```rust
let _ = writeln!(out, "server_name {server_name};");                        // proxy.rs:28
let _ = writeln!(out, "    location {path_prefix} {{");                     // proxy.rs:30
let _ = writeln!(out, "        proxy_pass http://{}:{};", upstream_host, upstream_port); // proxy.rs:31
```

## Steps to reproduce

1. In the dashboard, open an agent's **Proxy routes** tab and create a route.
2. For `server_name`, submit:

   ```
   evil.com;
   location /etc-secret { alias /etc/; return 200; }
   ```

3. Wait for the next reconciliation cycle (up to one heartbeat interval). The agent renders this into the Nginx config:

   ```nginx
   server {
       listen 80;
       server_name evil.com;
       location /etc-secret { alias /etc/; return 200; }
       location / {
           proxy_pass http://127.0.0.1:8080;
       }
   }
   ```

4. `nginx -t` passes (it's valid syntax), nginx reloads, and `curl http://<agent-host>/etc-secret/passwd` returns the contents of `/etc/passwd`.

The same applies via the API directly — `PUT /agents/{id}/proxy-routes/{name}` with a crafted JSON body. (No privileged account needed; any authenticated user who owns the agent can trigger this.)

## Expected behavior

Invalid values should be rejected client-side with `400 Bad Request` before they are ever stored or reach the agent. Specifically:

- `server_name` → only `[a-zA-Z0-9._-]`, optional leading `*.` wildcard, ≤255 chars
- `path_prefix` → must start with `/`, only URL-safe chars, ≤255 chars
- `upstream_host` → only `[a-zA-Z0-9.-]`, ≤255 chars

Anything containing `;`, `{`, `}`, whitespace, quotes, or newlines → reject.

## Suggested fix

1. **Primary (control plane):** add a validation function in `crates/control-plane/src/http.rs` and return `400 Bad Request` from `put_proxy_route` for any disallowed field — same pattern as the existing Compose stack name validation at `http.rs:560-567`.
2. **Defense-in-depth (agent):** optionally sanitize/reject bad values in `crates/agent/src/proxy.rs:render_route` as well, so a route can never produce an unsafe config even if it bypasses the API.

## Verification

- `cargo build -p harbory-control-plane` and `cargo test -p harbory-control-plane` (unit tests for the validator + an integration test asserting a malicious route returns `400`).
- Manual: attempt to create a route with each rejected character and confirm `400`; confirm a normal route (`app.example.com`, `/`, `127.0.0.1`) still works.