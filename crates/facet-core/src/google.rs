//! Google: what the App tab shows for each account state, the OAuth 2.0 installed-app flow with PKCE and a
//! loopback redirect, and reading Google's JSON replies.
//!
//! Everything here is pure: it builds and reads text. The network, the browser and the secret store are ports
//! (`crate::port`), used by `crate::google_flow`. JSON is read with sqlite's JSON functions.

use std::path::Path;

use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};

/// Where the authorization request is sent.
pub const AUTHORIZATION_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
/// Where codes and refresh tokens are exchanged for access tokens.
pub const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
/// The four non-sensitive scopes (`docs/google-oauth-setup.md`).
pub const SCOPES: [&str; 4] = [
    "openid",
    "https://www.googleapis.com/auth/userinfo.email",
    "https://www.googleapis.com/auth/userinfo.profile",
    "https://www.googleapis.com/auth/calendar.app.created",
];
/// How long a sign-in waits for the browser to come back, in seconds.
pub const SIGN_IN_TIMEOUT_SECONDS: u64 = 300;

/// The OAuth client this build signs in with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credentials {
    pub client_id: String,
    pub client_secret: String,
}

impl Credentials {
    /// Reads a console download: its `installed` object's `client_id` and `client_secret`. `None` for
    /// anything else, including a `web` client.
    pub fn from_json(json: &str) -> Option<Credentials> {
        let (id, secret) = json_pair(json, "$.installed.client_id", "$.installed.client_secret")?;
        Some(Credentials { client_id: id?, client_secret: secret? })
    }

    /// The first usable credentials of: the file `override_file` names (a leading `~` meaning `home`), the
    /// file `config_file`, and `bundled`. `None` when none is usable, which is a build without Google.
    pub fn resolve(
        override_file: Option<&str>,
        home: Option<&Path>,
        config_file: Option<&Path>,
        bundled: Option<&str>,
    ) -> Option<Credentials> {
        let from_file =
            |path: &Path| std::fs::read_to_string(path).ok().and_then(|json| Credentials::from_json(&json));
        if let Some(named) = override_file {
            let path = match (named.strip_prefix('~'), home) {
                (Some(rest), Some(home)) => home.join(rest.trim_start_matches('/')),
                _ => Path::new(named).to_path_buf(),
            };
            if let Some(found) = from_file(&path) {
                return Some(found);
            }
        }
        if let Some(found) = config_file.and_then(from_file) {
            return Some(found);
        }
        bundled.and_then(Credentials::from_json)
    }
}

/// The OAuth client compiled into this build by `scripts/generate-credentials.sh`, as the console's download.
/// `None` in a build made without one.
pub fn bundled_credentials() -> Option<&'static str> {
    #[cfg(facet_bundled_google)]
    return Some(include_str!(env!("FACET_BUNDLED_GOOGLE_CLIENT")));
    #[cfg(not(facet_bundled_google))]
    None
}

/// Two string fields of a JSON document, each `None` when absent or not a string. `None` overall when the
/// text is not JSON.
fn json_pair(json: &str, first: &str, second: &str) -> Option<(Option<String>, Option<String>)> {
    let connection = Connection::open_in_memory().ok()?;
    connection
        .query_row(
            "SELECT CASE WHEN json_valid(?1) THEN 1 ELSE 0 END, \
                    CASE json_type(?1, ?2) WHEN 'text' THEN json_extract(?1, ?2) END, \
                    CASE json_type(?1, ?3) WHEN 'text' THEN json_extract(?1, ?3) END",
            params![json, first, second],
            |row| {
                Ok((
                    row.get::<_, bool>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .ok()
        .and_then(|(valid, a, b)| valid.then_some((a, b)))
}

/// One string field of a JSON document. `None` when the text is not JSON or the field is not a string.
pub fn json_text(json: &str, path: &str) -> Option<String> {
    json_pair(json, path, path).and_then(|(value, _)| value)
}

/// `bytes` in base64url without padding.
pub fn base64url(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for index in 0..=chunk.len() {
            out.push(char::from(ALPHABET[((n >> (18 - 6 * index)) & 63) as usize]));
        }
    }
    out
}

/// Decodes base64url, padded or not. `None` for any character outside the alphabet.
pub fn from_base64url(text: &str) -> Option<Vec<u8>> {
    let value = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'-' | b'+' => 62,
            b'_' | b'/' => 63,
            _ => return None,
        } as u32)
    };
    let digits: Vec<u32> = text.trim_end_matches('=').bytes().map(value).collect::<Option<_>>()?;
    let mut out = Vec::with_capacity(digits.len() * 3 / 4);
    for chunk in digits.chunks(4) {
        let n = chunk.iter().enumerate().fold(0u32, |n, (i, d)| n | d << (18 - 6 * i));
        for index in 0..chunk.len().saturating_sub(1) {
            out.push((n >> (16 - 8 * index)) as u8);
        }
    }
    Some(out)
}

/// `text` percent-encoded for a URL query or a form body: everything but unreserved characters escaped.
pub fn percent_encode(text: &str) -> String {
    text.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => char::from(b).to_string(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}

/// Decodes percent escapes and `+` as a space. Invalid escapes are kept as they are.
pub fn percent_decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'+' => out.push(b' '),
            b'%' if index + 2 < bytes.len() => match u8::from_str_radix(&text[index + 1..index + 3], 16) {
                Ok(byte) => {
                    out.push(byte);
                    index += 2;
                }
                Err(_) => out.push(b'%'),
            },
            other => out.push(other),
        }
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// `name=value` pairs joined with `&`, each value percent-encoded.
pub fn form(pairs: &[(&str, &str)]) -> String {
    pairs
        .iter()
        .map(|(name, value)| format!("{name}={}", percent_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

/// The PKCE pair: a 43-character verifier from 32 random bytes and its S256 challenge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pkce {
    pub verifier: String,
    pub challenge: String,
}

impl Pkce {
    /// A pair from `random`, which must be 32 bytes from a cryptographic source.
    pub fn from_bytes(random: [u8; 32]) -> Pkce {
        let verifier = base64url(&random);
        let challenge = base64url(&Sha256::digest(verifier.as_bytes()));
        Pkce { verifier, challenge }
    }

    /// A fresh pair. An error when the system has no random source.
    pub fn new() -> Result<Pkce, String> {
        let mut random = [0u8; 32];
        getrandom::fill(&mut random).map_err(|error| format!("no random bytes: {error}"))?;
        Ok(Pkce::from_bytes(random))
    }
}

/// A fresh `state` value: 16 random bytes in base64url.
pub fn new_state() -> Result<String, String> {
    let mut random = [0u8; 16];
    getrandom::fill(&mut random).map_err(|error| format!("no random bytes: {error}"))?;
    Ok(base64url(&random))
}

/// The URL the browser is sent to, asking for offline access and the consent screen.
pub fn authorization_url(client_id: &str, redirect: &str, pkce: &Pkce, state: &str) -> String {
    let scope = SCOPES.join(" ");
    format!(
        "{AUTHORIZATION_ENDPOINT}?{}",
        form(&[
            ("client_id", client_id),
            ("redirect_uri", redirect),
            ("response_type", "code"),
            ("scope", &scope),
            ("code_challenge", &pkce.challenge),
            ("code_challenge_method", "S256"),
            ("state", state),
            ("access_type", "offline"),
            ("prompt", "consent"),
        ])
    )
}

/// What one request arriving at the loopback listener means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Redirect {
    Code(String),
    Denied(String),
    /// Not this sign-in's redirect: another state, no code, or something like a favicon request.
    Ignored,
}

/// Reads a request line such as `GET /?state=...&code=... HTTP/1.1`.
pub fn redirect(request_line: &str, expected_state: &str) -> Redirect {
    let Some(target) = request_line.split(' ').nth(1) else { return Redirect::Ignored };
    let Some((_, query)) = target.split_once('?') else { return Redirect::Ignored };
    let value = |name: &str| {
        query.split('&').find_map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            (key == name).then(|| percent_decode(value))
        })
    };
    if value("state").as_deref() != Some(expected_state) {
        return Redirect::Ignored;
    }
    if let Some(error) = value("error") {
        return Redirect::Denied(error);
    }
    match value("code") {
        Some(code) if !code.is_empty() => Redirect::Code(code),
        _ => Redirect::Ignored,
    }
}

/// The HTTP response the browser gets after the redirect: a small page saying `heading`.
pub fn redirect_response(heading: &str) -> String {
    let html = format!(
        "<!doctype html><meta charset=\"utf-8\"><title>Facet</title>\
         <style>body{{font:17px -apple-system,system-ui,sans-serif;margin:4rem auto;max-width:26rem;color:#1c1d21}}</style>\
         <h1 style=\"font-size:1.4rem\">{heading}</h1>\
         <p style=\"color:#55575e\">You can close this tab and go back to Facet.</p>"
    );
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{html}",
        html.len()
    )
}

/// What a token endpoint reply carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub name: Option<String>,
    pub email: Option<String>,
}

/// Reads a token endpoint reply. Name and email come from the `id_token`'s payload, which is not verified:
/// it arrived over TLS from Google in answer to this request. `None` without an `access_token`.
pub fn tokens(json: &str) -> Option<Tokens> {
    let access_token = json_text(json, "$.access_token")?;
    let claims = json_text(json, "$.id_token").and_then(|token| {
        let payload = token.split('.').nth(1)?;
        String::from_utf8(from_base64url(payload)?).ok()
    });
    let claim = |name: &str| claims.as_deref().and_then(|payload| json_text(payload, &format!("$.{name}")));
    Some(Tokens {
        access_token,
        refresh_token: json_text(json, "$.refresh_token"),
        name: claim("name"),
        email: claim("email"),
    })
}

/// Whether there is an account on record, from `google_account`'s name and email. Blank counts as absent.
pub fn has_google_identity(name: Option<&str>, email: Option<&str>) -> bool {
    [name, email].into_iter().flatten().any(|value| !value.trim().is_empty())
}

/// What the secret store says about the refresh token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialState {
    Present,
    Missing,
    /// The store would not answer.
    Unavailable(String),
}

/// What Google last said about the saved sign-in. Never stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoogleSignInState {
    NotAsked,
    Working,
    Refused(String),
    Unreachable(String),
}

/// The account as the App tab shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoogleAccountState {
    NotConnected,
    SignedOut,
    Unreadable,
    Unverified,
    Connected,
    Expired(String),
    Unreachable(String),
}

/// The account state from what is on record, what the store says and what Google last said.
pub fn account_state(
    has_google_identity: bool,
    credential: &CredentialState,
    sign_in: &GoogleSignInState,
) -> GoogleAccountState {
    if !has_google_identity {
        return GoogleAccountState::NotConnected;
    }
    match credential {
        CredentialState::Missing => GoogleAccountState::SignedOut,
        CredentialState::Unavailable(_) => GoogleAccountState::Unreadable,
        CredentialState::Present => match sign_in {
            GoogleSignInState::NotAsked => GoogleAccountState::Unverified,
            GoogleSignInState::Working => GoogleAccountState::Connected,
            GoogleSignInState::Refused(reason) => GoogleAccountState::Expired(reason.clone()),
            GoogleSignInState::Unreachable(reason) => GoogleAccountState::Unreachable(reason.clone()),
        },
    }
}

/// What the Google section draws for an account state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub status: &'static str,
    /// Whether the button disconnects rather than signs in.
    pub disconnects: bool,
    pub button_text: &'static str,
    pub button_label: &'static str,
    pub is_button_enabled: bool,
    pub shows_calendar_row: bool,
    pub note: String,
}

/// The Google section for `state`, with or without credentials in this build and while a sign-in runs.
pub fn section(state: &GoogleAccountState, has_google_credentials: bool, is_signing_in: bool) -> Section {
    use GoogleAccountState as S;
    let status = match state {
        S::NotConnected => "Not connected",
        S::SignedOut => "Signed out",
        S::Unreadable => "Cannot be checked",
        S::Unverified | S::Connected => "Connected",
        S::Expired(_) => "Sign-in expired",
        S::Unreachable(_) => "Connected, not checked",
    };
    let disconnects = matches!(state, S::Unreadable | S::Unverified | S::Connected | S::Unreachable(_));
    let with_reason = |reason: &str| if reason.is_empty() { String::new() } else { format!(" ({reason})") };
    let offers_sign_in = matches!(state, S::NotConnected | S::SignedOut | S::Expired(_));
    let note = if offers_sign_in && !has_google_credentials {
        "This copy of Facet was built without Google credentials, so it cannot sign in.".to_string()
    } else {
        match state {
            S::NotConnected => {
                "Opens your browser to approve. Facet only ever touches a calendar it makes itself, and \
                                never the ones you already have."
                    .to_string()
            }
            S::SignedOut => {
                "The account is remembered but its sign-in is not. Signing in again restores it.".to_string()
            }
            S::Expired(reason) => format!(
                "Google would not accept the saved sign-in{}. Signing in again restores it.",
                with_reason(reason)
            ),
            S::Unreachable(reason) => format!(
                "Facet could not reach Google to check this{}, so it is showing what it has.",
                with_reason(reason)
            ),
            S::Unreadable => {
                "Facet could not read its sign-in from your Keychain, so it cannot say whether this works."
                    .to_string()
            }
            S::Unverified | S::Connected => String::new(),
        }
    };
    Section {
        status,
        disconnects,
        button_text: if is_signing_in {
            "Signing in..."
        } else if disconnects {
            "Disconnect"
        } else {
            "Sign in with Google"
        },
        button_label: if disconnects { "Account" } else { "Google" },
        is_button_enabled: !is_signing_in && (disconnects || has_google_credentials),
        shows_calendar_row: matches!(state, S::Unverified | S::Connected | S::Unreachable(_)),
        note,
    }
}

/// The calendar name to send for a typed one: trimmed, and `Facet` when that leaves nothing.
pub fn calendar_name(typed: &str) -> String {
    match typed.trim() {
        "" => "Facet".to_string(),
        name => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64url_round_trips_and_matches_the_rfc_example() {
        // RFC 7636 appendix B.
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(
            base64url(&Sha256::digest(verifier.as_bytes())),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
        for text in ["", "f", "fo", "foo", "foob", "fooba", "foobar"] {
            assert_eq!(from_base64url(&base64url(text.as_bytes())).expect("decodes"), text.as_bytes());
        }
        assert_eq!(base64url(b"foobar"), "Zm9vYmFy");
        assert_eq!(base64url(b"fo"), "Zm8");
    }

    #[test]
    fn a_verifier_is_43_characters_with_its_s256_challenge() {
        let pkce = Pkce::from_bytes([7; 32]);
        assert_eq!(pkce.verifier.len(), 43);
        assert_eq!(pkce.challenge, base64url(&Sha256::digest(pkce.verifier.as_bytes())));
        assert_ne!(Pkce::new().expect("random").verifier, Pkce::new().expect("random").verifier);
        assert_eq!(new_state().expect("random").len(), 22);
    }

    #[test]
    fn the_authorization_url_asks_for_offline_consent_with_every_scope() {
        let pkce = Pkce::from_bytes([1; 32]);
        let url = authorization_url("id.apps", "http://127.0.0.1:5555", &pkce, "abc");
        assert!(url.starts_with("https://accounts.google.com/o/oauth2/v2/auth?client_id=id.apps&"));
        assert!(url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A5555"));
        assert!(url.contains("scope=openid%20https%3A%2F%2Fwww.googleapis.com%2Fauth%2Fuserinfo.email"));
        assert!(url.contains("calendar.app.created"));
        assert!(url.contains(&format!("code_challenge={}&code_challenge_method=S256", pkce.challenge)));
        assert!(url.contains("access_type=offline&prompt=consent"));
    }

    #[test]
    fn a_redirect_is_read_only_when_its_state_matches() {
        assert_eq!(
            redirect("GET /?state=abc&code=4%2F0AX HTTP/1.1", "abc"),
            Redirect::Code("4/0AX".to_string())
        );
        assert_eq!(
            redirect("GET /?state=abc&error=access_denied HTTP/1.1", "abc"),
            Redirect::Denied("access_denied".into())
        );
        assert_eq!(redirect("GET /?state=other&code=x HTTP/1.1", "abc"), Redirect::Ignored);
        assert_eq!(redirect("GET /favicon.ico HTTP/1.1", "abc"), Redirect::Ignored);
        assert!(redirect_response("Signed in").contains("You can close this tab and go back to Facet."));
    }

    #[test]
    fn a_token_reply_carries_the_identity_from_the_id_token() {
        let payload = base64url(br#"{"email":"a@b.test","name":"Ann"}"#);
        let json = format!(r#"{{"access_token":"at","refresh_token":"rt","id_token":"h.{payload}.s"}}"#);
        assert_eq!(
            tokens(&json),
            Some(Tokens {
                access_token: "at".into(),
                refresh_token: Some("rt".into()),
                name: Some("Ann".into()),
                email: Some("a@b.test".into())
            })
        );
        assert_eq!(tokens(r#"{"error":"invalid_grant"}"#), None);
        assert_eq!(tokens("not json"), None);
    }

    #[test]
    fn credentials_come_from_the_override_then_the_config_file_then_the_build() {
        let installed = r#"{"installed":{"client_id":"cid","client_secret":"cs"}}"#;
        assert_eq!(
            Credentials::from_json(installed),
            Some(Credentials { client_id: "cid".into(), client_secret: "cs".into() })
        );
        assert_eq!(Credentials::from_json(r#"{"web":{"client_id":"x","client_secret":"y"}}"#), None);
        let dir = std::env::temp_dir().join(format!("facet-google-creds-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let config = dir.join("config.json");
        std::fs::write(&config, r#"{"installed":{"client_id":"from-config","client_secret":"s"}}"#)
            .expect("write");
        let found = |override_file: Option<&str>, bundled: Option<&str>| {
            Credentials::resolve(override_file, Some(&dir), Some(&config), bundled).map(|c| c.client_id)
        };
        assert_eq!(found(None, Some(installed)).as_deref(), Some("from-config"));
        assert_eq!(found(Some("~/missing.json"), None).as_deref(), Some("from-config"));
        std::fs::remove_file(&config).expect("remove");
        assert_eq!(found(None, Some(installed)).as_deref(), Some("cid"));
        assert_eq!(found(None, None), None);
        std::fs::remove_dir(&dir).expect("remove dir");
    }

    #[test]
    fn each_state_draws_its_status_button_and_note() {
        use GoogleAccountState as S;
        let state = |identity, credential, sign_in| account_state(identity, &credential, &sign_in);
        assert_eq!(state(false, CredentialState::Present, GoogleSignInState::Working), S::NotConnected);
        assert_eq!(state(true, CredentialState::Missing, GoogleSignInState::NotAsked), S::SignedOut);
        assert_eq!(
            state(true, CredentialState::Unavailable("-25300".into()), GoogleSignInState::NotAsked),
            S::Unreadable
        );
        assert_eq!(state(true, CredentialState::Present, GoogleSignInState::NotAsked), S::Unverified);
        assert_eq!(
            state(true, CredentialState::Present, GoogleSignInState::Refused("x".into())),
            S::Expired("x".into())
        );

        let not_connected = section(&S::NotConnected, true, false);
        assert_eq!(
            (not_connected.status, not_connected.button_text, not_connected.button_label),
            ("Not connected", "Sign in with Google", "Google")
        );
        assert!(not_connected.is_button_enabled && !not_connected.shows_calendar_row);
        assert!(not_connected.note.starts_with("Opens your browser"));

        let no_credentials = section(&S::NotConnected, false, false);
        assert!(!no_credentials.is_button_enabled);
        assert_eq!(
            no_credentials.note,
            "This copy of Facet was built without Google credentials, so it cannot sign in."
        );

        let connected = section(&S::Connected, false, false);
        assert_eq!(
            (connected.status, connected.button_text, connected.button_label),
            ("Connected", "Disconnect", "Account")
        );
        assert!(connected.is_button_enabled && connected.shows_calendar_row && connected.note.is_empty());

        let signing_in = section(&S::NotConnected, true, true);
        assert_eq!(signing_in.button_text, "Signing in...");
        assert!(!signing_in.is_button_enabled);

        assert_eq!(
            section(&S::Expired("invalid_grant".into()), true, false).note,
            "Google would not accept the saved sign-in (invalid_grant). Signing in again restores it."
        );
        assert_eq!(section(&S::Unreachable(String::new()), true, false).status, "Connected, not checked");
    }

    #[test]
    fn identity_and_calendar_names_ignore_blanks() {
        assert!(!has_google_identity(Some("  "), None));
        assert!(has_google_identity(None, Some("a@b.test")));
        assert_eq!(calendar_name("  "), "Facet");
        assert_eq!(calendar_name(" Work "), "Work");
    }
}
