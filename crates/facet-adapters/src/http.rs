//! [`Http`] through `ureq`, with rustls. Every status comes back as a reply.

use std::time::Duration;

use facet_core::port::{Http, HttpResponse};

pub struct UreqHttp {
    agent: ureq::Agent,
}

impl UreqHttp {
    /// A client giving each request 30 seconds.
    pub fn new() -> UreqHttp {
        let config = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(30)))
            .build();
        UreqHttp { agent: config.into() }
    }
}

impl Default for UreqHttp {
    fn default() -> Self {
        UreqHttp::new()
    }
}

impl Http for UreqHttp {
    fn send(
        &self,
        method: &str,
        url: &str,
        bearer: Option<&str>,
        body: Option<(&str, &str)>,
    ) -> Result<HttpResponse, String> {
        let authorization = bearer.map(|token| format!("Bearer {token}"));
        let reply = match (method, body) {
            ("GET", _) => with_bearer(self.agent.get(url), authorization.as_deref()).call(),
            ("DELETE", _) => with_bearer(self.agent.delete(url), authorization.as_deref()).call(),
            ("POST", Some((kind, text))) => with_bearer(self.agent.post(url), authorization.as_deref())
                .header("Content-Type", kind)
                .send(text),
            ("PATCH", Some((kind, text))) => with_bearer(self.agent.patch(url), authorization.as_deref())
                .header("Content-Type", kind)
                .send(text),
            ("POST", None) => with_bearer(self.agent.post(url), authorization.as_deref()).send_empty(),
            (other, _) => return Err(format!("{other} is not a request this client sends")),
        };
        let mut reply = reply.map_err(|error| error.to_string())?;
        let status = reply.status().as_u16();
        let body = reply
            .body_mut()
            .read_to_string()
            .map_err(|error| format!("the reply could not be read: {error}"))?;
        Ok(HttpResponse { status, body })
    }
}

fn with_bearer<B>(request: ureq::RequestBuilder<B>, authorization: Option<&str>) -> ureq::RequestBuilder<B> {
    match authorization {
        Some(value) => request.header("Authorization", value),
        None => request,
    }
}
