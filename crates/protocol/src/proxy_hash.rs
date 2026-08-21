//! A canonical hash of a set of proxy routes. Both the control plane
//! (hashing desired state from the DB) and the agent (hashing whatever it
//! just applied) must compute the *identical* value for the same logical
//! route set, or the reconciliation-trigger comparison in
//! `crates/control-plane/src/stream.rs` breaks silently — hence this
//! lives here as the one shared implementation rather than being
//! reimplemented on each side.

use sha2::{Digest, Sha256};

use crate::v1::ProxyRoute;

fn write_field(hasher: &mut Sha256, bytes: &[u8]) {
    // Length-prefixed so e.g. name="a" + server_name="bc" can never hash
    // the same as name="ab" + server_name="c".
    hasher.update((bytes.len() as u32).to_le_bytes());
    hasher.update(bytes);
}

/// Order-independent: routes are sorted by name before hashing, since
/// "the same desired set, reported/stored in a different order" must
/// still be considered converged.
pub fn hash_routes(routes: &[ProxyRoute]) -> [u8; 32] {
    let mut sorted: Vec<&ProxyRoute> = routes.iter().collect();
    sorted.sort_by(|a, b| a.name.cmp(&b.name));

    let mut hasher = Sha256::new();
    hasher.update((sorted.len() as u32).to_le_bytes());
    for r in sorted {
        write_field(&mut hasher, r.name.as_bytes());
        write_field(&mut hasher, r.server_name.as_bytes());
        write_field(&mut hasher, &r.listen_port.to_le_bytes());
        write_field(&mut hasher, r.path_prefix.as_bytes());
        write_field(&mut hasher, r.upstream_host.as_bytes());
        write_field(&mut hasher, &r.upstream_port.to_le_bytes());
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(name: &str) -> ProxyRoute {
        ProxyRoute {
            name: name.into(),
            server_name: "example.test".into(),
            listen_port: 80,
            path_prefix: "/".into(),
            upstream_host: "127.0.0.1".into(),
            upstream_port: 8080,
        }
    }

    #[test]
    fn same_routes_same_hash() {
        assert_eq!(hash_routes(&[route("a")]), hash_routes(&[route("a")]));
    }

    #[test]
    fn order_does_not_matter() {
        let a = hash_routes(&[route("a"), route("b")]);
        let b = hash_routes(&[route("b"), route("a")]);
        assert_eq!(a, b);
    }

    #[test]
    fn different_routes_different_hash() {
        assert_ne!(hash_routes(&[route("a")]), hash_routes(&[route("b")]));
    }

    #[test]
    fn field_boundary_is_not_ambiguous() {
        let mut a = route("a");
        a.name = "a".into();
        a.server_name = "bc".into();
        let mut b = route("b");
        b.name = "ab".into();
        b.server_name = "c".into();
        assert_ne!(hash_routes(&[a]), hash_routes(&[b]));
    }

    #[test]
    fn empty_set_is_stable() {
        assert_eq!(hash_routes(&[]), hash_routes(&[]));
    }
}
