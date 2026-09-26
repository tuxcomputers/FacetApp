//! The App tab's Google section: shows the account's state and carries out sign-in, verification,
//! disconnecting, and making, renaming and deleting the calendar.
//!
//! The network calls run on a background thread; their outcomes come back to the UI thread through a
//! channel a timer drains, and are acted on there. What Google last said about the sign-in is held here for
//! as long as the app runs and is never stored. Everything else is read from `google_account` and the secret
//! store when the section is drawn.

use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::rc::{Rc, Weak};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use facet_core::database;
use facet_core::debug_log::{Record, Tag, Trace, plain};
use facet_core::google::{self, CredentialState, Credentials, GoogleSignInState, Pkce, Tokens};
use facet_core::google_flow::{self, CalendarFailure, SignInFailure, TokenFailure};
use facet_core::port::{Http, LoopbackListener, Opener, SecretLookup, SecretStore};
use rusqlite::Connection;
use slint::ComponentHandle;

use crate::notice::Notice;
use crate::{AppData, SettingsWindow};

/// What a background job came back with.
enum Outcome {
    SignedIn(Result<Tokens, SignInFailure>),
    Verified(Result<(), TokenFailure>),
    Confirmed(Result<String, Failure>),
    Created(Result<(String, String), Failure>),
    Renamed(Result<String, Failure>),
    Deleted { name: String, result: Result<(), Failure> },
}

/// A calendar job's failure: getting a token, or the calendar call itself.
enum Failure {
    Token(TokenFailure),
    NoToken(String),
    Calendar(CalendarFailure),
}

impl Failure {
    fn describe(&self) -> String {
        match self {
            Failure::Token(failure) => failure.describe(),
            Failure::NoToken(reason) => reason.clone(),
            Failure::Calendar(failure) => failure.describe(),
        }
    }

    fn is_gone(&self) -> bool {
        matches!(self, Failure::Calendar(CalendarFailure::Gone))
    }
}

/// The ports and settings a background job needs, all shareable across threads.
#[derive(Clone)]
struct Remote {
    http: Arc<dyn Http>,
    store: Arc<dyn SecretStore>,
    credentials: Option<Credentials>,
}

impl Remote {
    /// A fresh access token from the stored refresh token.
    fn access_token(&self) -> Result<String, Failure> {
        let credentials = self
            .credentials
            .as_ref()
            .ok_or_else(|| Failure::NoToken(SignInFailure::NoCredentials.describe()))?;
        match self.store.look_up() {
            SecretLookup::Found(refresh) => {
                google_flow::refresh(&*self.http, credentials, &refresh).map_err(Failure::Token)
            }
            SecretLookup::Missing => Err(Failure::NoToken("there is no saved sign-in".into())),
            SecretLookup::Unavailable(reason) => {
                Err(Failure::NoToken(format!("the Keychain would not answer ({reason})")))
            }
        }
    }
}

/// The Google section, attached to one Settings window.
pub struct Google {
    ui: slint::Weak<SettingsWindow>,
    database: PathBuf,
    log: Rc<Trace>,
    notice: Rc<Notice>,
    opener: Rc<dyn Opener>,
    listener: Arc<dyn LoopbackListener>,
    remote: Remote,
    sign_in: RefCell<GoogleSignInState>,
    is_signing_in: Cell<bool>,
    is_calendar_busy: Cell<bool>,
    sender: Sender<Outcome>,
    receiver: Receiver<Outcome>,
    pump: slint::Timer,
    this: RefCell<Weak<Google>>,
}

impl Google {
    /// Wires the section's callbacks on `ui`. `credentials` is `None` in a build without Google.
    #[allow(clippy::too_many_arguments)]
    pub fn attach(
        ui: &SettingsWindow,
        database: PathBuf,
        log: Rc<Trace>,
        notice: Rc<Notice>,
        opener: Rc<dyn Opener>,
        store: Arc<dyn SecretStore>,
        http: Arc<dyn Http>,
        listener: Arc<dyn LoopbackListener>,
        credentials: Option<Credentials>,
    ) -> Rc<Google> {
        let (sender, receiver) = channel();
        let google = Rc::new(Google {
            ui: ui.as_weak(),
            database,
            log,
            notice,
            opener,
            listener,
            remote: Remote { http, store, credentials },
            sign_in: RefCell::new(GoogleSignInState::NotAsked),
            is_signing_in: Cell::new(false),
            is_calendar_busy: Cell::new(false),
            sender,
            receiver,
            pump: slint::Timer::default(),
            this: RefCell::new(Weak::new()),
        });
        *google.this.borrow_mut() = Rc::downgrade(&google);

        let data = ui.global::<AppData>();
        let action = |run: fn(&Google)| {
            let weak = Rc::downgrade(&google);
            move || {
                if let Some(google) = weak.upgrade() {
                    run(&google);
                }
            }
        };
        data.on_google_button_pressed(action(Google::button_pressed));
        data.on_calendar_create_pressed(action(Google::create_calendar));
        data.on_calendar_delete_pressed(action(Google::delete_calendar));
        data.on_calendar_rename_opened(action(Google::open_rename));
        let weak = Rc::downgrade(&google);
        data.on_calendar_rename_committed(move |typed| {
            if let Some(google) = weak.upgrade() {
                google.rename_calendar(&typed);
            }
        });
        google
    }

    /// Draws the section and, when there is an account and a saved sign-in, asks Google whether it still
    /// works. Call when the App tab is shown.
    pub fn open(&self) {
        let credential = self.draw().map(|(credential, _)| credential);
        if credential == Some(CredentialState::Present) && !self.is_signing_in.get() {
            self.verify();
        }
    }

    /// Draws the section from `google_account`, the secret store and what Google last said. Returns what the
    /// store said and the account state; `None` when the section could not be read. The store is asked only
    /// when there is an account.
    fn draw(&self) -> Option<(CredentialState, google::GoogleAccountState)> {
        let ui = self.ui.upgrade()?;
        let connection = self.connect()?;
        let account = self.report(google_flow::account(&connection))?;
        let has_identity = account.has_google_identity();
        let credential = if has_identity {
            match self.remote.store.look_up() {
                SecretLookup::Found(_) => CredentialState::Present,
                SecretLookup::Missing => CredentialState::Missing,
                SecretLookup::Unavailable(reason) => CredentialState::Unavailable(reason),
            }
        } else {
            CredentialState::Missing
        };
        let state = google::account_state(has_identity, &credential, &self.sign_in.borrow());
        let section = google::section(&state, self.remote.credentials.is_some(), self.is_signing_in.get());
        let data = ui.global::<AppData>();
        data.set_google_status(section.status.into());
        data.set_google_name(account.name.clone().unwrap_or_default().into());
        data.set_google_email(account.email.clone().unwrap_or_default().into());
        data.set_google_button_label(section.button_label.into());
        data.set_google_button_text(section.button_text.into());
        data.set_google_button_enabled(section.is_button_enabled);
        data.set_shows_calendar_row(section.shows_calendar_row);
        data.set_has_calendar(account.calendar_id.is_some());
        data.set_calendar_name(account.calendar_name.clone().unwrap_or_default().into());
        data.set_calendar_enabled(!self.is_signing_in.get() && !self.is_calendar_busy.get());
        data.set_google_note(section.note.into());
        Some((credential, state))
    }

    /// Runs `work` on a background thread and acts on its outcome on the UI thread.
    fn run(&self, work: impl FnOnce(Remote) -> Outcome + Send + 'static) {
        let sender = self.sender.clone();
        let remote = self.remote.clone();
        std::thread::spawn(move || {
            // A closed channel means the section has gone, and the outcome has nobody to go to.
            if sender.send(work(remote)).is_err() {
                eprintln!("facet: a Google outcome arrived after the Settings window had gone");
            }
        });
        if !self.pump.running() {
            let weak = self.this.borrow().clone();
            self.pump.start(slint::TimerMode::Repeated, Duration::from_millis(200), move || {
                if let Some(google) = weak.upgrade() {
                    google.drain();
                }
            });
        }
    }

    fn drain(&self) {
        while let Ok(outcome) = self.receiver.try_recv() {
            self.finish(outcome);
        }
        if !self.is_signing_in.get() && !self.is_calendar_busy.get() {
            self.pump.stop();
        }
    }

    fn verify(&self) {
        let Some(credentials) = self.remote.credentials.clone() else {
            *self.sign_in.borrow_mut() = GoogleSignInState::Refused(SignInFailure::NoCredentials.describe());
            self.draw();
            return;
        };
        self.run(move |remote| {
            Outcome::Verified(match remote.store.look_up() {
                SecretLookup::Found(refresh) => {
                    google_flow::refresh(&*remote.http, &credentials, &refresh).map(|_| ())
                }
                SecretLookup::Missing => Err(TokenFailure::Refused("there is no saved sign-in".into())),
                SecretLookup::Unavailable(reason) => Err(TokenFailure::Unreachable(reason)),
            })
        });
    }

    /// Disconnects or signs in, whichever the button offers for the state as it reads now.
    fn button_pressed(&self) {
        let Some((_, state)) = self.draw() else { return };
        if google::section(&state, self.remote.credentials.is_some(), self.is_signing_in.get()).disconnects {
            self.disconnect();
        } else {
            self.sign_in();
        }
    }

    fn sign_in(&self) {
        let Some(credentials) = self.remote.credentials.clone() else {
            self.notice.tell("Facet could not connect to Google", &SignInFailure::NoCredentials.describe());
            return;
        };
        self.log.record(Tag::Google, || "Google sign-in started".to_string());
        let started = (|| -> Result<_, SignInFailure> {
            let session = self.listener.bind().map_err(SignInFailure::ListenerFailed)?;
            let redirect = format!("http://127.0.0.1:{}", session.port());
            let pkce = Pkce::new().map_err(SignInFailure::ListenerFailed)?;
            let state = google::new_state().map_err(SignInFailure::ListenerFailed)?;
            let url = google::authorization_url(&credentials.client_id, &redirect, &pkce, &state);
            self.opener.open_url(&url).map_err(|error| {
                SignInFailure::ListenerFailed(format!("the browser would not open: {error}"))
            })?;
            Ok((session, redirect, pkce, state))
        })();
        let (mut session, redirect, pkce, state) = match started {
            Ok(started) => started,
            Err(failure) => return self.signed_in(Err(failure)),
        };
        self.is_signing_in.set(true);
        self.draw();
        self.run(move |remote| {
            let result = google_flow::wait_for_code(
                &mut *session,
                &state,
                Duration::from_secs(google::SIGN_IN_TIMEOUT_SECONDS),
            )
            .and_then(|code| {
                google_flow::exchange_code(&*remote.http, &credentials, &code, &pkce.verifier, &redirect)
            });
            Outcome::SignedIn(result)
        });
    }

    fn finish(&self, outcome: Outcome) {
        match outcome {
            Outcome::SignedIn(result) => {
                self.is_signing_in.set(false);
                self.signed_in(result);
            }
            Outcome::Verified(result) => {
                let state = match result {
                    Ok(()) => {
                        self.log.record(Tag::Google, || "Google sign-in checked and works".to_string());
                        GoogleSignInState::Working
                    }
                    Err(TokenFailure::Refused(reason)) => {
                        self.log.record(Tag::Google, || {
                            format!("Google sign-in checked and refused: {}", plain(&reason))
                        });
                        GoogleSignInState::Refused(reason)
                    }
                    Err(TokenFailure::Unreachable(reason)) => {
                        self.log.record(Tag::Google, || {
                            format!("Google sign-in could not be checked: {}", plain(&reason))
                        });
                        GoogleSignInState::Unreachable(reason)
                    }
                };
                *self.sign_in.borrow_mut() = state;
            }
            Outcome::Confirmed(result) => self.confirmed(result),
            Outcome::Created(result) => {
                self.is_calendar_busy.set(false);
                match result {
                    Ok((id, name)) => {
                        let stored = self.write("calendar_id", &id) && self.write("calendar_name", &name);
                        self.log.record(Tag::Google, || {
                            format!(
                                "Google calendar created, {}{}",
                                plain(&name),
                                if stored { "" } else { " REFUSED" }
                            )
                        });
                    }
                    Err(failure) => self.calendar_failed("Facet could not make its calendar", &failure),
                }
            }
            Outcome::Renamed(result) => {
                self.is_calendar_busy.set(false);
                match result {
                    Ok(name) => {
                        let stored = self.write("calendar_name", &name);
                        self.log.record(Tag::Google, || {
                            format!(
                                "Google calendar renamed to {}{}",
                                plain(&name),
                                if stored { "" } else { " REFUSED" }
                            )
                        });
                    }
                    Err(failure) if failure.is_gone() => self.forget_calendar(),
                    Err(failure) => self.calendar_failed("Facet could not rename its calendar", &failure),
                }
            }
            Outcome::Deleted { name, result } => {
                self.is_calendar_busy.set(false);
                match result {
                    Ok(()) => {
                        let cleared = self.write("calendar_id", "") && self.write("calendar_name", "");
                        self.log.record(Tag::Google, || {
                            format!(
                                "Google calendar deleted, {}{}",
                                plain(&name),
                                if cleared { "" } else { " REFUSED" }
                            )
                        });
                    }
                    Err(failure) => self.calendar_failed("Facet could not delete its calendar", &failure),
                }
            }
        }
        self.draw();
    }

    fn signed_in(&self, result: Result<Tokens, SignInFailure>) {
        let result = result.and_then(|tokens| {
            let refresh = tokens.refresh_token.clone().unwrap_or_default();
            match self.remote.store.store(&refresh) {
                Ok(true) => Ok(tokens),
                Ok(false) => Err(SignInFailure::NoStore("it did not read back".into())),
                Err(reason) => Err(SignInFailure::NoStore(reason)),
            }
        });
        match result {
            Ok(tokens) => {
                let email = tokens.email.clone().unwrap_or_default();
                let stored = self.write("name", tokens.name.as_deref().unwrap_or_default())
                    && self.write("email", &email);
                *self.sign_in.borrow_mut() = GoogleSignInState::Working;
                self.log.record(Tag::Google, || {
                    format!(
                        "Google sign-in finished, account {}{}",
                        plain(&email),
                        if stored { "" } else { " REFUSED" }
                    )
                });
                self.settle_calendar();
            }
            Err(failure) => {
                self.log
                    .record(Tag::Google, || format!("Google sign-in failed: {}", plain(&failure.describe())));
                self.notice.tell("Facet could not connect to Google", &failure.describe());
            }
        }
        self.draw();
    }

    /// After a sign-in: confirms the calendar on record with Google, or says there is none.
    fn settle_calendar(&self) {
        let Some(connection) = self.connect() else { return };
        let Some(account) = self.report(google_flow::account(&connection)) else { return };
        let Some(id) = account.calendar_id else {
            self.log
                .record(Tag::Google, || "Google account connected with no calendar, none made".to_string());
            return;
        };
        self.is_calendar_busy.set(true);
        self.run(move |remote| {
            Outcome::Confirmed(remote.access_token().and_then(|token| {
                google_flow::calendar(&*remote.http, &token, &id).map_err(Failure::Calendar)
            }))
        });
    }

    fn confirmed(&self, result: Result<String, Failure>) {
        self.is_calendar_busy.set(false);
        match result {
            Ok(name) => {
                let stored = self.write("calendar_name", &name);
                self.log.record(Tag::Google, || {
                    format!(
                        "Google calendar confirmed, {}{}",
                        plain(&name),
                        if stored { "" } else { " REFUSED" }
                    )
                });
            }
            Err(failure) if failure.is_gone() => self.forget_calendar(),
            Err(failure) => self.log.record(Tag::Google, || {
                format!("Google calendar could not be confirmed: {}", plain(&failure.describe()))
            }),
        }
    }

    /// Forgets a calendar Google no longer has, and offers to make a new one.
    fn forget_calendar(&self) {
        let forgotten = self.write("calendar_id", "") && self.write("calendar_name", "");
        self.log.record(Tag::Google, || {
            format!(
                "Google calendar no longer resolves, forgotten{}",
                if forgotten { "" } else { " REFUSED" }
            )
        });
        let this = self.this.borrow().clone();
        self.notice.ask(
            "Facet cannot find its calendar",
            "The calendar Facet made is no longer in your Google account. It may have been deleted there.\n\nFacet \
             can make a new one and fill it from your recorded time.",
            &["Create Calendar", "Not Now"],
            move |index| {
                if index == 0
                    && let Some(google) = this.upgrade()
                {
                    google.create_calendar();
                }
            },
        );
        self.draw();
    }

    fn disconnect(&self) {
        let cleared = self.write("name", "") && self.write("email", "");
        self.log.record(Tag::Google, || {
            format!(
                "Google account disconnected{}",
                if cleared { "" } else { " REFUSED, the table still holds an identity" }
            )
        });
        if !cleared {
            self.notice.tell(
                "That setting was not saved",
                "The database would not take the new value for \u{201c}Google account\u{201d}, so the setting is \
                 unchanged and the row has gone back to what is stored.\n\nNothing else has been affected. Trying \
                 again is safe.",
            );
            self.draw();
            return;
        }
        if let Err(reason) = self.remote.store.clear() {
            self.log.record_failure(Tag::Google, || {
                format!("The saved Google sign-in could not be removed: {reason}")
            });
        }
        *self.sign_in.borrow_mut() = GoogleSignInState::NotAsked;
        self.draw();
    }

    fn create_calendar(&self) {
        self.is_calendar_busy.set(true);
        self.draw();
        self.run(|remote| {
            Outcome::Created(remote.access_token().and_then(|token| {
                google_flow::create_calendar(&*remote.http, &token, "Facet").map_err(Failure::Calendar)
            }))
        });
    }

    fn open_rename(&self) {
        let Some(ui) = self.ui.upgrade() else { return };
        let data = ui.global::<AppData>();
        data.set_calendar_typed(data.get_calendar_name());
        data.set_calendar_editing(true);
    }

    fn rename_calendar(&self, typed: &str) {
        let Some(ui) = self.ui.upgrade() else { return };
        let data = ui.global::<AppData>();
        data.set_calendar_editing(false);
        let name = google::calendar_name(typed);
        let Some(connection) = self.connect() else { return };
        let Some(account) = self.report(google_flow::account(&connection)) else { return };
        let (Some(id), current) = (account.calendar_id, account.calendar_name) else { return };
        if current.as_deref() == Some(name.as_str()) {
            return;
        }
        self.is_calendar_busy.set(true);
        self.draw();
        self.run(move |remote| {
            Outcome::Renamed(remote.access_token().and_then(|token| {
                google_flow::rename_calendar(&*remote.http, &token, &id, &name).map_err(Failure::Calendar)
            }))
        });
    }

    fn delete_calendar(&self) {
        let Some(connection) = self.connect() else { return };
        let Some(account) = self.report(google_flow::account(&connection)) else { return };
        let (Some(id), Some(name)) = (account.calendar_id, account.calendar_name) else { return };
        let this = self.this.borrow().clone();
        let asked_about = name.clone();
        self.notice.ask(
            &format!("Delete the \u{201c}{name}\u{201d} calendar?"),
            "This deletes the calendar from your Google account, along with every event Facet has written to it. It \
             cannot be undone from here.\n\nYour recorded time is not affected: it stays in Facet, and a new calendar \
             can be made and filled from it.",
            &["Cancel", "Delete Calendar"],
            move |index| {
                let Some(google) = this.upgrade() else { return };
                if index != 1 {
                    google.log.record(Tag::Google, || {
                        format!("Button clicked: Cancel, calendar {} not deleted", plain(&asked_about))
                    });
                    return;
                }
                google.is_calendar_busy.set(true);
                google.draw();
                google.run(move |remote| Outcome::Deleted {
                    name,
                    result: remote.access_token().and_then(|token| {
                        google_flow::delete_calendar(&*remote.http, &token, &id).map_err(Failure::Calendar)
                    }),
                });
            },
        );
    }

    fn calendar_failed(&self, title: &str, failure: &Failure) {
        self.log.record(Tag::Google, || format!("{title}: {}", plain(&failure.describe())));
        self.notice.tell(title, &failure.describe());
    }

    /// Writes one field of `google_account`, returning whether the table holds it.
    fn write(&self, field: &str, value: &str) -> bool {
        self.connect()
            .and_then(|connection| {
                self.report(google_flow::write_field(&connection, field, value, &*self.log))
            })
            .unwrap_or(false)
    }

    fn connect(&self) -> Option<Connection> {
        match database::connect(&self.database) {
            Ok(connection) => Some(connection),
            Err(error) => {
                self.log.record_failure(Tag::Database, || {
                    format!("Google: the database would not open: {error}")
                });
                None
            }
        }
    }

    fn report<T>(&self, result: Result<T, rusqlite::Error>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(error) => {
                self.log.record_failure(Tag::Database, || format!("Google: a database call failed: {error}"));
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use facet_core::port::{HttpResponse, LoopbackSession};
    use slint::platform::software_renderer::MinimalSoftwareWindow;
    use slint::platform::{Platform, WindowAdapter};
    use std::sync::Mutex;

    struct Headless(Rc<MinimalSoftwareWindow>);
    impl Platform for Headless {
        fn create_window_adapter(&self) -> Result<Rc<dyn WindowAdapter>, slint::PlatformError> {
            Ok(self.0.clone())
        }
    }

    /// Answers by URL and method, like a small Google.
    struct FakeGoogle;
    impl Http for FakeGoogle {
        fn send(
            &self,
            method: &str,
            url: &str,
            _bearer: Option<&str>,
            body: Option<(&str, &str)>,
        ) -> Result<HttpResponse, String> {
            let reply = |status: u16, body: &str| Ok(HttpResponse { status, body: body.to_string() });
            let text = body.map(|(_, text)| text).unwrap_or_default();
            if url == google::TOKEN_ENDPOINT && text.contains("authorization_code") {
                let claims = google::base64url(br#"{"email":"ann@example.test","name":"Ann"}"#);
                return reply(
                    200,
                    &format!(r#"{{"access_token":"a","refresh_token":"r","id_token":"h.{claims}.s"}}"#),
                );
            }
            if url == google::TOKEN_ENDPOINT {
                return reply(200, r#"{"access_token":"a"}"#);
            }
            match method {
                "POST" => reply(200, r#"{"id":"cal@group.calendar.google.com","summary":"Facet"}"#),
                "PATCH" => reply(200, r#"{"summary":"Work"}"#),
                "DELETE" => reply(204, ""),
                _ => reply(200, r#"{"summary":"Facet"}"#),
            }
        }
    }

    struct FakeStore(Mutex<Option<String>>);
    impl SecretStore for FakeStore {
        fn store(&self, secret: &str) -> Result<bool, String> {
            *self.0.lock().expect("lock") = Some(secret.to_string());
            Ok(true)
        }
        fn look_up(&self) -> SecretLookup {
            match self.0.lock().expect("lock").clone() {
                Some(secret) => SecretLookup::Found(secret),
                None => SecretLookup::Missing,
            }
        }
        fn clear(&self) -> Result<(), String> {
            *self.0.lock().expect("lock") = None;
            Ok(())
        }
    }

    /// Hands the state from the opened URL straight back as a redirect with a code.
    struct FakeBrowser(Arc<Mutex<Option<String>>>);
    impl Opener for FakeBrowser {
        fn reveal(&self, _file: &std::path::Path) -> Result<(), String> {
            Ok(())
        }
        fn open_url(&self, url: &str) -> Result<(), String> {
            let state =
                url.split('&').find_map(|pair| pair.strip_prefix("state=")).expect("a state").to_string();
            *self.0.lock().expect("lock") = Some(state);
            Ok(())
        }
    }

    struct FakeListener(Arc<Mutex<Option<String>>>);
    impl LoopbackListener for FakeListener {
        fn bind(&self) -> Result<Box<dyn LoopbackSession>, String> {
            Ok(Box::new(FakeSession(Arc::clone(&self.0))))
        }
    }
    struct FakeSession(Arc<Mutex<Option<String>>>);
    impl LoopbackSession for FakeSession {
        fn port(&self) -> u16 {
            5555
        }
        fn next(
            &mut self,
            _timeout: Duration,
            respond: &dyn Fn(&str) -> String,
        ) -> Result<Option<String>, String> {
            let state = self.0.lock().expect("lock").clone().expect("the browser was opened first");
            let line = format!("GET /?state={state}&code=c HTTP/1.1");
            respond(&line);
            Ok(Some(line))
        }
    }

    /// Drains outcomes until nothing is running, for up to five seconds.
    fn settle(google: &Google) {
        for _ in 0..100 {
            std::thread::sleep(Duration::from_millis(50));
            google.drain();
            if !google.is_signing_in.get() && !google.is_calendar_busy.get() {
                std::thread::sleep(Duration::from_millis(50));
                google.drain();
                return;
            }
        }
        panic!("the Google job did not finish");
    }

    #[test]
    fn signing_in_making_renaming_deleting_and_disconnecting() {
        slint::platform::set_platform(Box::new(Headless(MinimalSoftwareWindow::new(Default::default()))))
            .expect("the headless platform should install");
        let path = std::env::temp_dir().join(format!("facet-ui-google-{}.sqlite", std::process::id()));
        if path.exists() {
            std::fs::remove_file(&path).expect("a stale test database should be removable");
        }
        let connection = database::open(&path, database::APPDATA_DDL).expect("the app DDL should apply");
        let ui = SettingsWindow::new().expect("the window should build");
        let notice = Notice::attach(&ui);
        let redirect = Arc::new(Mutex::new(None));
        let store = Arc::new(FakeStore(Mutex::new(None)));
        let attach = |credentials: Option<Credentials>| {
            Google::attach(
                &ui,
                path.clone(),
                Rc::new(Trace::none()),
                Rc::clone(&notice),
                Rc::new(FakeBrowser(Arc::clone(&redirect))),
                Arc::clone(&store) as Arc<dyn SecretStore>,
                Arc::new(FakeGoogle),
                Arc::new(FakeListener(Arc::clone(&redirect))),
                credentials,
            )
        };
        let data = ui.global::<AppData>();

        let without = attach(None);
        without.open();
        assert_eq!(data.get_google_status(), "Not connected");
        assert!(!data.get_google_button_enabled());
        assert!(data.get_google_note().contains("built without Google credentials"));
        drop(without);

        let google = attach(Some(Credentials { client_id: "cid".into(), client_secret: "cs".into() }));
        google.open();
        assert_eq!(data.get_google_button_text(), "Sign in with Google");
        google.button_pressed();
        settle(&google);
        assert_eq!(notice.title(), "");
        assert_eq!(data.get_google_status(), "Connected");
        assert_eq!(data.get_google_email(), "ann@example.test");
        assert_eq!(data.get_google_button_text(), "Disconnect");
        assert!(data.get_shows_calendar_row() && !data.get_has_calendar());
        assert_eq!(store.look_up(), SecretLookup::Found("r".into()));

        google.create_calendar();
        settle(&google);
        assert!(data.get_has_calendar());
        assert_eq!(data.get_calendar_name(), "Facet");

        google.rename_calendar("  Work ");
        settle(&google);
        assert_eq!(data.get_calendar_name(), "Work");

        google.delete_calendar();
        assert_eq!(notice.title(), "Delete the \u{201c}Work\u{201d} calendar?");
        notice.choose(1);
        settle(&google);
        assert!(!data.get_has_calendar());

        google.button_pressed();
        assert_eq!(data.get_google_status(), "Not connected");
        assert_eq!(store.look_up(), SecretLookup::Missing);
        let account = google_flow::account(&connection).expect("should read");
        assert_eq!(account.email, None);

        std::fs::remove_file(&path).expect("the test database should be removable");
    }
}
