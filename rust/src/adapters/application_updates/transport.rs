use std::io::Read;
use std::time::Duration;

use reqwest::blocking::{Client, Response};
use reqwest::{StatusCode, Url, header, redirect};
use serde::de::DeserializeOwned;

use super::manifest::invalid;
use crate::domain::application_update::{ApplicationUpdateError, ApplicationUpdateFailureCode};

#[derive(Clone)]
pub(super) struct UpdateTransport {
    client: Client,
    #[cfg(test)]
    fixture_url: Option<Url>,
}

impl UpdateTransport {
    pub fn optional_json<T: DeserializeOwned>(
        &self,
        url: &str,
        limit: u64,
    ) -> Result<Option<T>, ApplicationUpdateError> {
        let response = self.request(url)?.send().map_err(|_| network())?;
        if response.status() == StatusCode::NOT_FOUND {
            return Ok(None);
        }
        Self::read_json(response, limit).map(Some)
    }
    pub fn new() -> Result<Self, ApplicationUpdateError> {
        let client = Client::builder()
            .https_only(true)
            .timeout(Duration::from_secs(10))
            .connect_timeout(Duration::from_secs(10))
            .pool_max_idle_per_host(0)
            .retry(reqwest::retry::never())
            .user_agent("Cedarflake-Ame-Updates/1")
            .redirect(redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 5 || !admitted_url(attempt.url()) {
                    attempt.error("Application update redirect was rejected")
                } else {
                    attempt.follow()
                }
            }))
            .build()
            .map_err(|_| network())?;
        Ok(Self {
            client,
            #[cfg(test)]
            fixture_url: None,
        })
    }

    #[cfg(test)]
    pub(super) fn fixture(address: std::net::SocketAddr) -> Self {
        assert!(address.ip().is_loopback());
        Self {
            client: Client::builder()
                .no_proxy()
                .timeout(Duration::from_secs(2))
                .redirect(redirect::Policy::none())
                .build()
                .unwrap(),
            fixture_url: Some(Url::parse(&format!("http://{address}/package")).unwrap()),
        }
    }

    pub fn json<T: DeserializeOwned>(
        &self,
        url: &str,
        limit: u64,
    ) -> Result<T, ApplicationUpdateError> {
        let response = self.request(url)?.send().map_err(|_| network())?;
        Self::read_json(response, limit)
    }

    fn read_json<T: DeserializeOwned>(
        response: Response,
        limit: u64,
    ) -> Result<T, ApplicationUpdateError> {
        if !response.status().is_success() {
            return Err(network());
        }
        if response.content_length().is_some_and(|size| size > limit) {
            return Err(invalid());
        }
        let mut bytes = Vec::new();
        response
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| network())?;
        if bytes.len() as u64 > limit {
            return Err(invalid());
        }
        serde_json::from_slice(&bytes).map_err(|_| invalid())
    }

    pub fn open(
        &self,
        url: &str,
        offset: u64,
        length: u64,
    ) -> Result<(Response, u64), ApplicationUpdateError> {
        let mut request = self.request(url)?;
        if offset > 0 {
            request = request.header(header::RANGE, format!("bytes={offset}-"));
        }
        let response = request.send().map_err(|_| network())?;
        let actual = response_offset(response.status(), response.headers(), offset, length)?;
        Ok((response, actual))
    }

    fn request(
        &self,
        url: &str,
    ) -> Result<reqwest::blocking::RequestBuilder, ApplicationUpdateError> {
        let url = Url::parse(url).map_err(|_| invalid())?;
        if !admitted_url(&url) {
            return Err(invalid());
        }
        #[cfg(test)]
        let url = self.fixture_url.clone().unwrap_or(url);
        Ok(self
            .client
            .get(url)
            .header(header::ACCEPT_ENCODING, "identity"))
    }
}

fn admitted_url(url: &Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && matches!(
            url.host_str(),
            Some(
                "api.github.com"
                    | "github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
            )
        )
}

fn response_offset(
    status: StatusCode,
    headers: &header::HeaderMap,
    requested: u64,
    length: u64,
) -> Result<u64, ApplicationUpdateError> {
    let actual = match status {
        StatusCode::OK if !headers.contains_key(header::CONTENT_RANGE) => 0,
        StatusCode::PARTIAL_CONTENT if requested > 0 && requested < length => {
            let expected = format!("bytes {requested}-{}/{length}", length - 1);
            if headers
                .get(header::CONTENT_RANGE)
                .and_then(|v| v.to_str().ok())
                != Some(expected.as_str())
            {
                return Err(invalid());
            }
            requested
        }
        _ => return Err(network()),
    };
    if headers.get(header::CONTENT_LENGTH).is_some_and(|value| {
        value.to_str().ok().and_then(|v| v.parse::<u64>().ok()) != Some(length - actual)
    }) || headers
        .get(header::CONTENT_ENCODING)
        .is_some_and(|value| value != "identity")
    {
        return Err(invalid());
    }
    Ok(actual)
}

pub(super) fn network() -> ApplicationUpdateError {
    ApplicationUpdateError::new(
        ApplicationUpdateFailureCode::Network,
        "The application update server could not be reached",
    )
}

#[cfg(test)]
mod tests;
