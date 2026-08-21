//! Dev-only: prints what `proxy::render` produces for a couple of sample
//! routes, so its output can be fed into a real nginx binary (`nginx -t`)
//! to verify it's actually valid config, not just structurally plausible.
//! Usage: cargo run -p harbory-agent --example render_proxy_config

use harbory_agent::proxy::render;
use harbory_protocol::v1::ProxyRoute;

fn main() {
    let routes = vec![
        ProxyRoute {
            name: "web".into(),
            server_name: "app.example.test".into(),
            listen_port: 8080,
            path_prefix: "/".into(),
            upstream_host: "127.0.0.1".into(),
            upstream_port: 9001,
        },
        ProxyRoute {
            name: "api".into(),
            server_name: "".into(), // catch-all
            listen_port: 8081,
            path_prefix: "/v1".into(),
            upstream_host: "127.0.0.1".into(),
            upstream_port: 9002,
        },
    ];

    print!("{}", render(&routes));
}
