//! Talks to GitHub's OAuth and REST APIs on behalf of a connected account.
//! No GitHub-specific state lives here — token storage is
//! `store/github.rs`'s job, this module is purely the HTTP client. Mirrors
//! `jwks.rs`'s style (plain `reqwest`, no shared client, this isn't a
//! hot path).

use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error)]
pub enum GitHubError {
    #[error("request to GitHub failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("GitHub rejected the request: {0}")]
    Api(String),
}

/// Builds the URL to send the browser to for step one of the OAuth
/// Authorization Code flow. `client_id` is not secret (it's meant to be
/// public — GitHub's own docs embed it in front-end JS), so this can run
/// entirely server-side without exposing anything a client-side redirect
/// wouldn't already.
pub fn oauth_authorize_url(client_id: &str, redirect_uri: &str, state: &str) -> String {
    // `repo` is the narrowest scope that still covers private repos —
    // GitHub OAuth Apps (unlike GitHub Apps) don't support finer-grained
    // per-repo scoping.
    format!(
        "https://github.com/login/oauth/authorize?client_id={client_id}&redirect_uri={redirect_uri}&scope=repo&state={state}",
        client_id = urlencoding_component(client_id),
        redirect_uri = urlencoding_component(redirect_uri),
        state = urlencoding_component(state),
    )
}

/// Percent-encodes just enough for a query-string value — avoids pulling
/// in a whole `url`/`urlencoding` crate dependency for the handful of
/// characters (`:`, `/`, spaces) that actually show up in a redirect URI
/// or a base64url state token.
fn urlencoding_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    error_description: Option<String>,
    error: Option<String>,
}

/// Step two: exchange the one-time `code` GitHub sent to the callback for
/// a real access token. GitHub's token endpoint returns 200 with an
/// `error` field on failure rather than a non-2xx status, so the actual
/// error check is on the parsed body, not `error_for_status`.
pub async fn exchange_code(
    client_id: &str,
    client_secret: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<String, GitHubError> {
    let client = reqwest::Client::new();
    let response: TokenResponse = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .await?
        .json()
        .await?;

    match response.access_token {
        Some(token) => Ok(token),
        None => Err(GitHubError::Api(
            response.error_description.or(response.error).unwrap_or_else(|| "no access_token in response".into()),
        )),
    }
}

#[derive(Deserialize)]
struct GitHubUserResponse {
    login: String,
}

/// Confirms the token actually works and gets the username to display in
/// the dashboard's "connected as ..." state.
pub async fn fetch_login(access_token: &str) -> Result<String, GitHubError> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {access_token}"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "harbory-control-plane")
        .send()
        .await?
        .error_for_status()
        .map_err(|err| GitHubError::Api(err.to_string()))?;

    let user: GitHubUserResponse = response.json().await?;
    Ok(user.login)
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GitHubRepo {
    pub full_name: String,
    pub private: bool,
    pub default_branch: String,
    pub html_url: String,
}

/// First 100 repos, most-recently-updated first — a real pagination loop
/// is a natural follow-up once someone actually has more than that, not
/// a v1 requirement.
pub async fn list_repos(access_token: &str) -> Result<Vec<GitHubRepo>, GitHubError> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://api.github.com/user/repos?per_page=100&sort=updated")
        .header("Authorization", format!("Bearer {access_token}"))
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "harbory-control-plane")
        .send()
        .await?
        .error_for_status()
        .map_err(|err| GitHubError::Api(err.to_string()))?;

    Ok(response.json().await?)
}
