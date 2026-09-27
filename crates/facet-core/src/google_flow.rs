//! Google's flows over the ports: waiting for the sign-in redirect, exchanging and refreshing tokens, the
//! calendar calls, and the `google_account` row.
//!
//! The network calls block, so a caller with a window runs them off its UI thread. The row is read and
//! written on the caller's own connection.

use std::time::{Duration, Instant};

use rusqlite::{Connection, OptionalExtension};

use crate::app_settings::{self, Value};
use crate::debug_log::Record;
use crate::google::{self, Credentials, Redirect, Tokens};
use crate::port::{Http, HttpResponse, LoopbackSession};

/// Why a sign-in did not finish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SignInFailure {
    NoCredentials,
    Denied(String),
    ListenerFailed(String),
    ExchangeFailed(String),
    NoRefreshToken,
    TimedOut,
    NoStore(String),
}

impl SignInFailure {
    /// The failure in words, for a notice.
    pub fn describe(&self) -> String {
        match self {
            SignInFailure::NoCredentials => {
                "This copy of Facet has no Google credentials built into it, so it cannot sign in.".to_string()
            }
            SignInFailure::Denied(reason) if reason == "access_denied" => {
                "Sign-in was cancelled at the Google page.".to_string()
            }
            SignInFailure::Denied(reason) => format!("Google refused the sign-in: {reason}."),
            SignInFailure::ListenerFailed(reason) => {
                format!("Facet could not open a local port to receive the sign-in: {reason}.")
            }
            SignInFailure::ExchangeFailed(reason) => format!("Google would not complete the sign-in: {reason}."),
            SignInFailure::NoRefreshToken => "Google did not return a refresh token, so the connection would stop \
                                             working within the hour. Removing Facet at \
                                             myaccount.google.com/permissions and signing in again usually fixes it."
                .to_string(),
            SignInFailure::TimedOut => "Sign-in was cancelled: nothing came back from the browser in five minutes.".to_string(),
            SignInFailure::NoStore(reason) => {
                format!("The sign-in worked, but Facet could not keep it in your Keychain: {reason}.")
            }
        }
    }
}

/// Waits on `session` until the redirect for `state` arrives or `timeout` passes, answering every request.
/// Returns the authorization code.
pub fn wait_for_code(
    session: &mut dyn LoopbackSession,
    state: &str,
    timeout: Duration,
) -> Result<String, SignInFailure> {
    let deadline = Instant::now() + timeout;
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(SignInFailure::TimedOut);
        }
        let respond = |line: &str| match google::redirect(line, state) {
            Redirect::Code(_) => google::redirect_response("Facet is connected to Google."),
            Redirect::Denied(_) => google::redirect_response("Facet was not connected to Google."),
            Redirect::Ignored => {
                "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n".to_string()
            }
        };
        let Some(line) = session.next(left, &respond).map_err(SignInFailure::ListenerFailed)? else {
            return Err(SignInFailure::TimedOut);
        };
        match google::redirect(&line, state) {
            Redirect::Code(code) => return Ok(code),
            Redirect::Denied(reason) => return Err(SignInFailure::Denied(reason)),
            Redirect::Ignored => {}
        }
    }
}

/// Exchanges an authorization code for tokens. A reply without a refresh token is a failure: the connection
/// would stop within the hour.
pub fn exchange_code(
    http: &dyn Http,
    credentials: &Credentials,
    code: &str,
    verifier: &str,
    redirect_uri: &str,
) -> Result<Tokens, SignInFailure> {
    let body = google::form(&[
        ("code", code),
        ("client_id", &credentials.client_id),
        ("client_secret", &credentials.client_secret),
        ("code_verifier", verifier),
        ("grant_type", "authorization_code"),
        ("redirect_uri", redirect_uri),
    ]);
    let reply = http
        .send("POST", google::TOKEN_ENDPOINT, None, Some(("application/x-www-form-urlencoded", &body)))
        .map_err(SignInFailure::ExchangeFailed)?;
    if reply.status != 200 {
        return Err(SignInFailure::ExchangeFailed(reason(&reply)));
    }
    let tokens = google::tokens(&reply.body)
        .ok_or_else(|| SignInFailure::ExchangeFailed("an unreadable reply".into()))?;
    if tokens.refresh_token.is_none() {
        return Err(SignInFailure::NoRefreshToken);
    }
    Ok(tokens)
}

/// Why an access token could not be had.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenFailure {
    /// Google answered and would not accept the refresh token.
    Refused(String),
    /// No answer, or a server error: the sign-in may still be good.
    Unreachable(String),
}

impl TokenFailure {
    pub fn describe(&self) -> String {
        match self {
            TokenFailure::Refused(reason) => format!("Google would not accept the saved sign-in ({reason})"),
            TokenFailure::Unreachable(reason) => format!("Google could not be reached ({reason})"),
        }
    }
}

/// A fresh access token from `refresh_token`.
pub fn refresh(
    http: &dyn Http,
    credentials: &Credentials,
    refresh_token: &str,
) -> Result<String, TokenFailure> {
    let body = google::form(&[
        ("client_id", &credentials.client_id),
        ("client_secret", &credentials.client_secret),
        ("refresh_token", refresh_token),
        ("grant_type", "refresh_token"),
    ]);
    let reply = http
        .send("POST", google::TOKEN_ENDPOINT, None, Some(("application/x-www-form-urlencoded", &body)))
        .map_err(TokenFailure::Unreachable)?;
    match reply.status {
        200 => google::tokens(&reply.body)
            .map(|tokens| tokens.access_token)
            .ok_or_else(|| TokenFailure::Unreachable("an unreadable reply".into())),
        500..=599 => Err(TokenFailure::Unreachable(reason(&reply))),
        _ => Err(TokenFailure::Refused(reason(&reply))),
    }
}

/// Google's reason for a non-200 reply: the token endpoint's `error`, the API's `error.message`, else the
/// status.
fn reason(reply: &HttpResponse) -> String {
    google::json_text(&reply.body, "$.error")
        .or_else(|| google::json_text(&reply.body, "$.error.message"))
        .unwrap_or_else(|| format!("HTTP {}", reply.status))
}

const CALENDARS: &str = "https://www.googleapis.com/calendar/v3/calendars";

/// Why a calendar call did not succeed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CalendarFailure {
    /// The calendar no longer exists (404 or 410).
    Gone,
    Failed(String),
}

impl CalendarFailure {
    pub fn describe(&self) -> String {
        match self {
            CalendarFailure::Gone => "the calendar no longer exists".to_string(),
            CalendarFailure::Failed(reason) => reason.clone(),
        }
    }
}

fn calendar_call(
    http: &dyn Http,
    method: &str,
    url: &str,
    token: &str,
    body: Option<&str>,
) -> Result<HttpResponse, CalendarFailure> {
    let reply = http
        .send(method, url, Some(token), body.map(|text| ("application/json", text)))
        .map_err(CalendarFailure::Failed)?;
    match reply.status {
        200..=299 => Ok(reply),
        404 | 410 => Err(CalendarFailure::Gone),
        _ => Err(CalendarFailure::Failed(reason(&reply))),
    }
}

/// The name Google holds for calendar `id`.
pub fn calendar(http: &dyn Http, token: &str, id: &str) -> Result<String, CalendarFailure> {
    let reply =
        calendar_call(http, "GET", &format!("{CALENDARS}/{}", google::percent_encode(id)), token, None)?;
    google::json_text(&reply.body, "$.summary")
        .ok_or_else(|| CalendarFailure::Failed("an unreadable reply".into()))
}

/// Creates a calendar called `name` and returns its id and name.
pub fn create_calendar(
    http: &dyn Http,
    token: &str,
    name: &str,
) -> Result<(String, String), CalendarFailure> {
    let body = json_object("summary", name);
    let reply = calendar_call(http, "POST", CALENDARS, token, Some(&body))?;
    let id = google::json_text(&reply.body, "$.id")
        .ok_or_else(|| CalendarFailure::Failed("no id in the reply".into()))?;
    let name = google::json_text(&reply.body, "$.summary").unwrap_or_else(|| name.to_string());
    Ok((id, name))
}

/// Renames calendar `id` to `name` and returns the name Google now holds.
pub fn rename_calendar(
    http: &dyn Http,
    token: &str,
    id: &str,
    name: &str,
) -> Result<String, CalendarFailure> {
    let body = json_object("summary", name);
    let url = format!("{CALENDARS}/{}", google::percent_encode(id));
    let reply = calendar_call(http, "PATCH", &url, token, Some(&body))?;
    Ok(google::json_text(&reply.body, "$.summary").unwrap_or_else(|| name.to_string()))
}

/// Deletes calendar `id`. A calendar already gone counts as deleted.
pub fn delete_calendar(http: &dyn Http, token: &str, id: &str) -> Result<(), CalendarFailure> {
    match calendar_call(http, "DELETE", &format!("{CALENDARS}/{}", google::percent_encode(id)), token, None) {
        Ok(_) | Err(CalendarFailure::Gone) => Ok(()),
        Err(failure) => Err(failure),
    }
}

/// `{"<key>": "<value>"}` with the value escaped, built by sqlite.
fn json_object(key: &str, value: &str) -> String {
    Connection::open_in_memory()
        .and_then(|connection| {
            connection.query_row("SELECT json_object(?1, ?2)", [key, value], |row| row.get(0))
        })
        .unwrap_or_else(|_| format!("{{\"{key}\":\"\"}}"))
}

/// What `google_account` holds. Blank fields read as `None`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Account {
    pub name: Option<String>,
    pub email: Option<String>,
    pub calendar_id: Option<String>,
    pub calendar_name: Option<String>,
}

impl Account {
    pub fn has_google_identity(&self) -> bool {
        google::has_google_identity(self.name.as_deref(), self.email.as_deref())
    }
}

/// Reads `google_account`. An absent row reads as empty.
pub fn account(connection: &Connection) -> Result<Account, rusqlite::Error> {
    let row = connection
        .query_row(
            "SELECT json_extract(setting_value, '$.name'), json_extract(setting_value, '$.email'), \
                    json_extract(setting_value, '$.calendar_id'), json_extract(setting_value, '$.calendar_name') \
             FROM setting WHERE setting_name = 'google_account'",
            [],
            |row| {
                Ok((
                    row.get::<_, Option<String>>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()?;
    let blank = |value: Option<String>| value.filter(|text| !text.trim().is_empty());
    Ok(row
        .map(|(name, email, calendar_id, calendar_name)| Account {
            name: blank(name),
            email: blank(email),
            calendar_id: blank(calendar_id),
            calendar_name: blank(calendar_name),
        })
        .unwrap_or_default())
}

/// Writes one text field of `google_account` and reads it back. Returns whether the table holds it.
pub fn write_field(
    connection: &Connection,
    field: &str,
    value: &str,
    log: &impl Record,
) -> Result<bool, rusqlite::Error> {
    app_settings::write(connection, "google_account", field, &Value::Text(value.to_string()), log)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::seeded;
    use std::sync::Mutex;

    const NO_LOG: Option<crate::debug_log::DebugLog> = None;

    /// Answers each request with the next canned reply and remembers what was asked.
    struct FakeHttp {
        replies: Mutex<Vec<Result<HttpResponse, String>>>,
        asked: Mutex<Vec<(String, String, Option<String>)>>,
    }

    impl FakeHttp {
        fn new(replies: Vec<Result<(u16, &str), &str>>) -> FakeHttp {
            FakeHttp {
                replies: Mutex::new(
                    replies
                        .into_iter()
                        .rev()
                        .map(|reply| {
                            reply
                                .map(|(status, body)| HttpResponse { status, body: body.into() })
                                .map_err(String::from)
                        })
                        .collect(),
                ),
                asked: Mutex::new(Vec::new()),
            }
        }
    }

    impl Http for FakeHttp {
        fn send(
            &self,
            method: &str,
            url: &str,
            _bearer: Option<&str>,
            body: Option<(&str, &str)>,
        ) -> Result<HttpResponse, String> {
            self.asked.lock().expect("lock").push((
                method.into(),
                url.into(),
                body.map(|(_, text)| text.into()),
            ));
            self.replies.lock().expect("lock").pop().expect("a reply was scripted")
        }
    }

    struct FakeSession(Vec<String>);

    impl LoopbackSession for FakeSession {
        fn port(&self) -> u16 {
            5555
        }
        fn next(
            &mut self,
            _timeout: Duration,
            respond: &dyn Fn(&str) -> String,
        ) -> Result<Option<String>, String> {
            let line = self.0.pop();
            if let Some(line) = &line {
                assert!(!respond(line).is_empty());
            }
            Ok(line)
        }
    }

    fn credentials() -> Credentials {
        Credentials { client_id: "cid".into(), client_secret: "cs".into() }
    }

    #[test]
    fn the_code_is_taken_from_the_matching_redirect_after_ignoring_others() {
        let mut session = FakeSession(vec![
            "GET /?state=s&code=the-code HTTP/1.1".into(),
            "GET /favicon.ico HTTP/1.1".into(),
        ]);
        assert_eq!(wait_for_code(&mut session, "s", Duration::from_secs(5)), Ok("the-code".into()));
        let mut denied = FakeSession(vec!["GET /?state=s&error=access_denied HTTP/1.1".into()]);
        let failure = wait_for_code(&mut denied, "s", Duration::from_secs(5)).expect_err("denied");
        assert_eq!(failure.describe(), "Sign-in was cancelled at the Google page.");
        let mut silent = FakeSession(Vec::new());
        assert_eq!(wait_for_code(&mut silent, "s", Duration::from_secs(5)), Err(SignInFailure::TimedOut));
    }

    #[test]
    fn an_exchange_needs_a_refresh_token() {
        let good = FakeHttp::new(vec![Ok((200, r#"{"access_token":"a","refresh_token":"r"}"#))]);
        let tokens = exchange_code(&good, &credentials(), "code", "verifier", "http://127.0.0.1:5555")
            .expect("tokens");
        assert_eq!(tokens.refresh_token.as_deref(), Some("r"));
        let asked = good.asked.lock().expect("lock");
        let body = asked[0].2.as_deref().expect("a body");
        assert!(body.contains("grant_type=authorization_code") && body.contains("code_verifier=verifier"));

        let no_refresh = FakeHttp::new(vec![Ok((200, r#"{"access_token":"a"}"#))]);
        assert_eq!(
            exchange_code(&no_refresh, &credentials(), "c", "v", "r"),
            Err(SignInFailure::NoRefreshToken)
        );
        let refused = FakeHttp::new(vec![Ok((400, r#"{"error":"invalid_grant"}"#))]);
        assert_eq!(
            exchange_code(&refused, &credentials(), "c", "v", "r"),
            Err(SignInFailure::ExchangeFailed("invalid_grant".into()))
        );
    }

    #[test]
    fn a_refresh_tells_refused_from_unreachable() {
        let ok = FakeHttp::new(vec![Ok((200, r#"{"access_token":"fresh"}"#))]);
        assert_eq!(refresh(&ok, &credentials(), "r"), Ok("fresh".into()));
        let refused = FakeHttp::new(vec![Ok((400, r#"{"error":"invalid_grant"}"#))]);
        assert_eq!(
            refresh(&refused, &credentials(), "r"),
            Err(TokenFailure::Refused("invalid_grant".into()))
        );
        let offline = FakeHttp::new(vec![Err("connection refused")]);
        assert_eq!(
            refresh(&offline, &credentials(), "r"),
            Err(TokenFailure::Unreachable("connection refused".into()))
        );
        let down = FakeHttp::new(vec![Ok((503, ""))]);
        assert_eq!(refresh(&down, &credentials(), "r"), Err(TokenFailure::Unreachable("HTTP 503".into())));
    }

    #[test]
    fn the_calendar_calls_encode_the_id_once_and_treat_gone_as_gone() {
        let http = FakeHttp::new(vec![
            Ok((200, r#"{"id":"abc@group.calendar.google.com","summary":"Facet"}"#)),
            Ok((200, r#"{"summary":"Work"}"#)),
            Ok((404, "")),
            Ok((404, "")),
        ]);
        assert_eq!(
            create_calendar(&http, "t", "Facet"),
            Ok(("abc@group.calendar.google.com".into(), "Facet".into()))
        );
        assert_eq!(rename_calendar(&http, "t", "abc@group.calendar.google.com", "Work"), Ok("Work".into()));
        assert_eq!(calendar(&http, "t", "abc@group.calendar.google.com"), Err(CalendarFailure::Gone));
        assert_eq!(delete_calendar(&http, "t", "abc@group.calendar.google.com"), Ok(()));
        let asked = http.asked.lock().expect("lock");
        assert_eq!(asked[0].2.as_deref(), Some(r#"{"summary":"Facet"}"#));
        assert_eq!(
            asked[1].1,
            "https://www.googleapis.com/calendar/v3/calendars/abc%40group.calendar.google.com"
        );
        assert_eq!(asked[1].0, "PATCH");
    }

    #[test]
    fn the_account_row_reads_blank_as_absent_and_takes_writes() {
        let connection = seeded();
        assert_eq!(account(&connection).expect("should read"), Account::default());
        assert!(write_field(&connection, "email", "a@b.test", &NO_LOG).expect("should write"));
        assert!(write_field(&connection, "name", " ", &NO_LOG).expect("should write"));
        let read = account(&connection).expect("should read");
        assert_eq!((read.email.as_deref(), read.name.as_deref()), (Some("a@b.test"), None));
        assert!(read.has_google_identity());
    }
}
