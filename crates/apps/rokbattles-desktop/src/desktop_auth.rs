//! External-browser sign-in using a short-lived loopback listener and PKCE.

use std::{
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use rand::RngExt;
use sha2::Digest;
use tauri::{AppHandle, Manager, Webview, webview::Cookie};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};
use tauri_plugin_opener::OpenerExt;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::{shell::ShellState, tray};

const LOGIN_TIMEOUT: Duration = Duration::from_secs(600);
const MAX_SESSION_AGE_SECONDS: i64 = 30 * 24 * 60 * 60;

#[derive(Default)]
pub(crate) struct AuthState {
    task: Mutex<Option<tauri::async_runtime::JoinHandle<()>>>,
    generation: AtomicU64,
}

#[derive(Debug)]
enum LoginError {
    Unreported(&'static str),
    CallbackReplied,
}

impl From<&'static str> for LoginError {
    fn from(message: &'static str) -> Self {
        Self::Unreported(message)
    }
}

fn show_login_error(app: &AppHandle, message: &'static str) {
    app.dialog()
        .message(message)
        .title("Sign-in failed")
        .kind(MessageDialogKind::Error)
        .show(|_| {});
}

pub(crate) fn start(app: &AppHandle) {
    let state = app.state::<AuthState>();
    let Ok(mut task) = state.task.lock() else {
        show_login_error(app, "Could not start sign-in. Please restart the app.");
        return;
    };

    let generation = state.generation.fetch_add(1, Ordering::Relaxed) + 1;
    if let Some(previous) = task.take() {
        previous.abort();
    }

    let app = app.clone();
    *task = Some(tauri::async_runtime::spawn(async move {
        let result = tokio::time::timeout(LOGIN_TIMEOUT, login(&app, generation)).await;
        if app.state::<AuthState>().generation.load(Ordering::Relaxed) != generation {
            return;
        }

        match result {
            Ok(Ok(()) | Err(LoginError::CallbackReplied)) => {}
            Ok(Err(LoginError::Unreported(message))) => show_login_error(&app, message),
            Err(_) => show_login_error(&app, "Sign-in timed out. Select Sign in to try again."),
        }
    }));
}

fn random_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);

    URL_SAFE_NO_PAD.encode(bytes)
}

fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(sha2::Sha256::digest(verifier.as_bytes()))
}

async fn login(app: &AppHandle, generation: u64) -> Result<(), LoginError> {
    let app_url = app.state::<ShellState>().app_url().clone();
    let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .await
        .map_err(|_error| "Could not listen for the sign-in callback.")?;
    let port = listener.local_addr().map_err(|_error| "Could not start sign-in.")?.port();

    let state = random_token();
    let verifier = random_token();
    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_error| "Could not start sign-in.")?;

    let response = client
        .get(
            app_url
                .join("/proxy/v1/auth/desktop/login")
                .map_err(|_error| "Invalid sign-in URL.")?,
        )
        .query(&[
            ("port", port.to_string()),
            ("state", state.clone()),
            ("code_challenge", challenge(&verifier)),
        ])
        .send()
        .await
        .map_err(|_error| "Could not reach the sign-in service. Please try again.")?;
    if response.status() == reqwest::StatusCode::NOT_FOUND
        || response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED
    {
        return Err("Desktop sign-in is not available on the server yet.".into());
    }

    let authorization = response
        .headers()
        .get(reqwest::header::LOCATION)
        .and_then(|header| header.to_str().ok())
        .and_then(|url| tauri::Url::parse(url).ok())
        .filter(|url| {
            response.status().is_redirection()
                && url.scheme() == "https"
                && url.host_str() == Some("discord.com")
                && url.port_or_known_default() == Some(443)
                && url.path() == "/oauth2/authorize"
        })
        .ok_or("The sign-in service returned an invalid response.")?;
    app.opener()
        .open_url(authorization.as_str(), None::<&str>)
        .map_err(|_error| "Could not open your browser. Please try again.")?;

    let (mut browser, code) = receive_callback(&listener, &state).await?;
    drop(listener);

    let result = async {
        let response = client
            .post(
                app_url
                    .join("/proxy/v1/auth/desktop/exchange")
                    .map_err(|_error| "Invalid sign-in URL.")?,
            )
            .json(&serde_json::json!({"code": code, "verifier": verifier}))
            .send()
            .await
            .map_err(|_error| "Could not complete sign-in. Please try again.")?;
        if !response.status().is_success() {
            return Err("Sign-in expired or was already used. Please try again.");
        }

        let header = response
            .headers()
            .get(reqwest::header::SET_COOKIE)
            .and_then(|header| header.to_str().ok())
            .ok_or("The sign-in service did not return a session.")?;
        let cookie = session_cookie(header, &app_url)?;

        if app.state::<AuthState>().generation.load(Ordering::Relaxed) != generation {
            return Err("Sign-in cancelled.");
        }

        let view = app.get_webview("app").ok_or("Open App and try signing in again.")?;
        view.set_cookie(cookie)
            .map_err(|_error| "Could not save the App session. Please try again.")?;
        verify_app_session(&view, generation).await?;

        view.reload().map_err(|_error| "Signed in. Switch to App to reload the page.")?;
        tray::show_main_window(app);

        Ok(())
    }
    .await;

    let message = match &result {
        Ok(()) => "You are signed in. Return to ROK Battles. You can close this tab.",
        Err(message) => message,
    };
    reply(&mut browser, if result.is_ok() { "200 OK" } else { "400 Bad Request" }, message).await;

    result.map_err(|_message| LoginError::CallbackReplied)
}

async fn verify_app_session(view: &Webview, generation: u64) -> Result<(), &'static str> {
    const ERROR: &str = "The App could not confirm the session. Please try signing in again.";

    // Check through the actual webview so cookie storage or transport failures
    // cannot produce a misleading success message. Only the HTTP status leaves JS.
    let key = format!("__rokbattlesLoginCheck{generation}");
    view.eval(format!(
        r#"window.{key} = 0;
        fetch('/proxy/v1/auth/me', {{
            credentials: 'include', cache: 'no-store', redirect: 'error',
            signal: AbortSignal.timeout(15000)
        }}).then(response => {{ window.{key} = response.status; }})
          .catch(() => {{ window.{key} = -1; }});"#
    ))
    .map_err(|_error| ERROR)?;

    let result = tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
            view.eval_with_callback(format!("window.{key}"), move |status| {
                // The receiving login task may have timed out or been cancelled.
                drop(sender.try_send(status));
            })
            .map_err(|_error| ERROR)?;

            let value = receiver.recv().await.ok_or(ERROR)?;
            match serde_json::from_str::<i32>(&value) {
                Ok(200) => return Ok(()),
                Ok(0) => {}
                _ => return Err(ERROR),
            }

            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await;

    if let Err(error) = view.eval(format!("delete window.{key}")) {
        eprintln!("[rokbattles] could not clean up session check: {error}");
    }

    result.map_err(|_error| ERROR)?
}

fn session_cookie(header: &str, app_url: &tauri::Url) -> Result<Cookie<'static>, &'static str> {
    let mut cookie =
        Cookie::parse(header.to_owned()).map_err(|_error| "The sign-in session was invalid.")?;
    if cookie.name() != "sid"
        || cookie.value().is_empty()
        || cookie.secure() != Some(true)
        || cookie.http_only() != Some(true)
        || cookie.path() != Some("/")
        || cookie.domain().is_some()
        || !cookie
            .max_age()
            .is_some_and(|age| (1..=MAX_SESSION_AGE_SECONDS).contains(&age.whole_seconds()))
    {
        return Err("The sign-in session was invalid.");
    }

    cookie.set_domain(app_url.host_str().ok_or("Invalid App domain.")?.to_owned());

    // WebKit does not send Secure cookies over HTTP localhost. The URL override
    // is loopback-only and absent from release builds; production stays Secure.
    #[cfg(debug_assertions)]
    if app_url.scheme() == "http"
        && matches!(app_url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
    {
        cookie.set_secure(false);
    }

    Ok(cookie)
}

#[derive(Debug, PartialEq, Eq)]
enum Callback {
    Code(String),
    Cancelled,
    Failed,
}

fn parse_callback(request: &str, port: u16, state: &str) -> Option<Callback> {
    let mut lines = request.split("\r\n");
    let mut first = lines.next()?.split_whitespace();
    if first.next()? != "GET" {
        return None;
    }

    let target = first.next()?;
    if !target.starts_with("/auth/callback?")
        || first.next()? != "HTTP/1.1"
        || first.next().is_some()
    {
        return None;
    }

    let hosts: Vec<_> = lines
        .filter_map(|line| line.split_once(':'))
        .filter(|(name, _)| name.eq_ignore_ascii_case("host"))
        .collect();
    if hosts.len() != 1 || hosts.first()?.1.trim() != format!("127.0.0.1:{port}") {
        return None;
    }

    let url = tauri::Url::parse(&format!("http://127.0.0.1:{port}{target}")).ok()?;
    let values: Vec<_> = url.query_pairs().collect();
    if values.iter().filter(|(key, _)| key == "state").count() != 1
        || !values.iter().any(|(key, value)| key == "state" && value == state)
    {
        return None;
    }

    if let Some((_, error)) = values.iter().find(|(key, _)| key == "error") {
        return Some(if error == "access_denied" { Callback::Cancelled } else { Callback::Failed });
    }

    let codes: Vec<_> = values.iter().filter(|(key, _)| key == "code").collect();
    if codes.len() != 1 {
        return None;
    }

    let code = &codes.first()?.1;
    ((43..=128).contains(&code.len())
        && code.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_')))
    .then(|| Callback::Code(code.to_string()))
}

async fn read_request(stream: &mut TcpStream) -> Option<String> {
    let mut request = Vec::new();
    let mut buffer = [0u8; 1024];

    while request.len() < 8192 {
        let count = stream.read(&mut buffer).await.ok()?;
        if count == 0 {
            return None;
        }

        request.extend_from_slice(buffer.get(..count)?);
        if request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
            return String::from_utf8(request).ok();
        }
    }

    None
}

async fn receive_callback(
    listener: &TcpListener,
    state: &str,
) -> Result<(TcpStream, String), LoginError> {
    let port =
        listener.local_addr().map_err(|_error| "Could not read the sign-in callback.")?.port();

    loop {
        let (mut stream, _) =
            listener.accept().await.map_err(|_error| "Could not receive the sign-in callback.")?;
        let request = tokio::time::timeout(Duration::from_secs(3), read_request(&mut stream))
            .await
            .ok()
            .flatten();

        match request.and_then(|request| parse_callback(&request, port, state)) {
            Some(Callback::Code(code)) => return Ok((stream, code)),
            Some(Callback::Cancelled) => {
                reply(&mut stream, "200 OK", "Sign-in cancelled. You can return to ROK Battles.")
                    .await;
                return Err(LoginError::CallbackReplied);
            }
            Some(Callback::Failed) => {
                reply(
                    &mut stream,
                    "400 Bad Request",
                    "Discord sign-in could not finish. Return to ROK Battles and try again.",
                )
                .await;
                return Err(LoginError::CallbackReplied);
            }
            None => {
                reply(&mut stream, "400 Bad Request", "Invalid sign-in callback.").await;
            }
        }
    }
}

async fn reply(stream: &mut TcpStream, status: &str, message: &str) {
    if let Err(error) = write_reply(stream, status, message).await {
        eprintln!("[rokbattles] browser closed the sign-in callback: {error}");
    }
}

async fn write_reply(stream: &mut TcpStream, status: &str, message: &str) -> std::io::Result<()> {
    let body = format!(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\"><title>ROK Battles</title><body><h1>ROK Battles</h1><p>{message}</p></body></html>"
    );
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: default-src 'none'; frame-ancestors 'none'\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );

    tokio::time::timeout(Duration::from_secs(3), stream.write_all(response.as_bytes())).await??;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(port: u16, query: &str) -> String {
        format!("GET /auth/callback?{query} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n")
    }

    #[test]
    fn callback_requires_the_exact_host_state_method_and_one_code() {
        let code = "c".repeat(86);
        let valid = request(54321, &format!("state=expected&code={code}"));

        assert_eq!(parse_callback(&valid, 54321, "expected"), Some(Callback::Code(code)));

        for invalid in [
            valid.replace("state=expected", "state=other"),
            valid.replace("state=expected", "state=expected&state=expected"),
            valid.replace("code=", "code=extra&code="),
            valid.replace("127.0.0.1:54321", "evil.test:54321"),
            valid.replace("127.0.0.1:54321", "127.0.0.1:54322"),
            valid.replace("GET ", "POST "),
            valid.replace("/auth/callback?", "/other?"),
        ] {
            assert_eq!(parse_callback(&invalid, 54321, "expected"), None);
        }

        assert_eq!(
            parse_callback(
                &request(54321, "state=expected&error=access_denied"),
                54321,
                "expected"
            ),
            Some(Callback::Cancelled)
        );
    }

    #[test]
    fn only_a_secure_scoped_session_cookie_is_accepted() {
        let valid = "sid=test_session; Path=/; Max-Age=2592000; HttpOnly; Secure; SameSite=Lax";
        let production = "https://rokbattles.com/".parse().expect("App URL");
        let cookie = session_cookie(valid, &production).expect("session");

        assert_eq!(cookie.domain(), Some("rokbattles.com"));
        assert_eq!(cookie.secure(), Some(true));
        assert_eq!(cookie.max_age().map(|age| age.whole_seconds()), Some(2592000));

        for invalid in [
            valid.replace("sid=", "other="),
            valid.replace("; HttpOnly", ""),
            valid.replace("; Secure", ""),
            valid.replace("Path=/", "Path=/other"),
            valid.replace("2592000", "2592001"),
            valid.replace("Max-Age=2592000; ", ""),
            valid.replace("2592000", "0"),
            format!("{valid}; Domain=evil.test"),
        ] {
            session_cookie(&invalid, &production).expect_err("invalid session");
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    fn local_http_development_keeps_session_httponly_without_secure_transport() {
        let local = "http://localhost:3000/".parse().expect("local URL");
        let cookie = session_cookie(
            "sid=test; Path=/; Max-Age=604800; HttpOnly; Secure; SameSite=Lax",
            &local,
        )
        .expect("local session");

        assert_eq!(cookie.domain(), Some("localhost"));
        assert_eq!(cookie.secure(), Some(false));
        assert_eq!(cookie.http_only(), Some(true));
    }

    #[test]
    fn pkce_matches_the_rfc_7636_test_vector() {
        assert_eq!(
            challenge("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk"),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );

        assert_ne!(random_token(), random_token());
    }

    #[tokio::test]
    async fn listener_ignores_unrelated_requests_then_accepts_the_matching_callback() {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await.expect("bind");
        let port = listener.local_addr().expect("address").port();
        let code = "x".repeat(86);
        let expected = code.clone();
        let task = tokio::spawn(async move {
            let (mut stream, code) =
                receive_callback(&listener, "expected").await.expect("callback");
            write_reply(&mut stream, "200 OK", "Success").await.expect("reply");
            code
        });

        let mut stray =
            TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).await.expect("connect");
        stray.write_all(request(port, "state=wrong&code=wrong").as_bytes()).await.expect("send");
        let mut response = String::new();
        stray.read_to_string(&mut response).await.expect("read");
        assert!(response.starts_with("HTTP/1.1 400"));

        let mut browser =
            TcpStream::connect((std::net::Ipv4Addr::LOCALHOST, port)).await.expect("connect");
        browser
            .write_all(request(port, &format!("state=expected&code={code}")).as_bytes())
            .await
            .expect("send");
        let mut response = String::new();
        browser.read_to_string(&mut response).await.expect("read");

        assert!(response.starts_with("HTTP/1.1 200"));
        assert!(response.contains("Cache-Control: no-store"));
        assert_eq!(task.await.expect("task"), expected);
        expect_listener_closed((std::net::Ipv4Addr::LOCALHOST, port).into()).await;
    }

    #[tokio::test]
    async fn cancelled_and_failed_callbacks_report_the_outcome_in_the_browser() {
        for (error, status, message) in [
            ("access_denied", "200 OK", "Sign-in cancelled."),
            ("oauth_failed", "400 Bad Request", "Discord sign-in could not finish."),
        ] {
            let listener =
                TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await.expect("bind");
            let address = listener.local_addr().expect("address");
            let task = tokio::spawn(async move { receive_callback(&listener, "expected").await });

            let mut browser = TcpStream::connect(address).await.expect("connect");
            browser
                .write_all(
                    request(address.port(), &format!("state=expected&error={error}")).as_bytes(),
                )
                .await
                .expect("send");

            let mut response = String::new();
            tokio::time::timeout(Duration::from_secs(2), browser.read_to_string(&mut response))
                .await
                .expect("callback responds promptly")
                .expect("read");

            assert!(response.starts_with(&format!("HTTP/1.1 {status}")));
            assert!(response.contains(message));
            assert!(matches!(task.await.expect("task"), Err(LoginError::CallbackReplied)));
        }
    }

    #[tokio::test]
    async fn aborting_login_closes_the_loopback_listener() {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0)).await.expect("bind");
        let address = listener.local_addr().expect("address");
        let task = tokio::spawn(async move { receive_callback(&listener, "expected").await });

        task.abort();

        assert!(task.await.expect_err("cancelled").is_cancelled());
        expect_listener_closed(address).await;
    }

    async fn expect_listener_closed(address: std::net::SocketAddr) {
        // Allow reactor cleanup and the refusal probe to finish. Windows can take
        // a few seconds to report ConnectionRefused even after the port is closed.
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                match TcpStream::connect(address).await {
                    Ok(stream) => drop(stream),
                    Err(error) => {
                        assert_eq!(
                            error.kind(),
                            std::io::ErrorKind::ConnectionRefused,
                            "unexpected error when probing {address}: {error}"
                        );
                        break;
                    }
                }

                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("listener stops accepting connections");
    }
}
