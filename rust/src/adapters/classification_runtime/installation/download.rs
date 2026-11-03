use std::io::Read;
use std::time::Duration;

use reqwest::blocking::{Client, Response};
use reqwest::{StatusCode, Url, redirect};

use crate::domain::ScanError;

use super::super::manifest::Artifact;

const DOWNLOAD_BASE: &str = "https://huggingface.co/onnx-community/siglip2-base-patch16-224-ONNX/resolve/ba1f3b0843f24bc5417d38e19c37b287d719b2f4";
const NETWORK_TIMEOUT: Duration = Duration::from_secs(10);

pub(super) trait ModelDownload {
    fn open(&mut self, artifact: &Artifact, offset: u64) -> Result<DownloadBody, ScanError>;
}

pub(super) struct DownloadBody {
    pub reader: Box<dyn Read>,
    pub offset: u64,
}

pub(super) struct HttpsModelDownload {
    client: Option<Client>,
}

impl HttpsModelDownload {
    pub fn new() -> Self {
        Self { client: None }
    }

    fn client(&mut self) -> Result<&Client, ScanError> {
        if self.client.is_none() {
            self.client = Some(
                Client::builder()
                    .https_only(true)
                    .timeout(NETWORK_TIMEOUT)
                    .connect_timeout(NETWORK_TIMEOUT)
                    .pool_max_idle_per_host(0)
                    .retry(reqwest::retry::never())
                    .user_agent("Cedarflake-Ame-model-installation/1")
                    .redirect(redirect::Policy::custom(|attempt| {
                        if attempt.previous().len() >= 5 || !admitted_url(attempt.url()) {
                            attempt.error("Model download redirect was rejected")
                        } else {
                            attempt.follow()
                        }
                    }))
                    .build()
                    .map_err(|_| network_error())?,
            );
        }
        self.client.as_ref().ok_or_else(network_error)
    }
}

impl ModelDownload for HttpsModelDownload {
    fn open(&mut self, artifact: &Artifact, offset: u64) -> Result<DownloadBody, ScanError> {
        let directory = if artifact.name.ends_with(".onnx") {
            "onnx/"
        } else {
            ""
        };
        let mut request = self
            .client()?
            .get(format!("{DOWNLOAD_BASE}/{directory}{}", artifact.name))
            .header(reqwest::header::ACCEPT_ENCODING, "identity");
        if offset > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={offset}-"));
        }
        let response = request.send().map_err(|_| network_error())?;
        let offset = admitted_response(&response, artifact, offset)?;
        Ok(DownloadBody {
            reader: Box::new(response),
            offset,
        })
    }
}

fn admitted_response(
    response: &Response,
    artifact: &Artifact,
    requested: u64,
) -> Result<u64, ScanError> {
    let offset = match response.status() {
        StatusCode::OK
            if !response
                .headers()
                .contains_key(reqwest::header::CONTENT_RANGE) =>
        {
            0
        }
        StatusCode::PARTIAL_CONTENT if requested > 0 && requested < artifact.length => {
            let expected = format!(
                "bytes {requested}-{}/{}",
                artifact.length - 1,
                artifact.length
            );
            if response
                .headers()
                .get(reqwest::header::CONTENT_RANGE)
                .and_then(|value| value.to_str().ok())
                != Some(expected.as_str())
            {
                return Err(response_error());
            }
            requested
        }
        _ => return Err(response_error()),
    };
    if response
        .content_length()
        .is_some_and(|length| length != artifact.length - offset)
        || response
            .headers()
            .get(reqwest::header::CONTENT_ENCODING)
            .is_some_and(|encoding| encoding != "identity")
    {
        return Err(response_error());
    }
    Ok(offset)
}

fn response_error() -> ScanError {
    ScanError::new(
        "classification_download_response_invalid",
        "The model server did not return the expected file",
    )
}

fn admitted_url(url: &Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && url.host_str().is_some_and(|host| {
            host == "huggingface.co"
                || host.ends_with(".huggingface.co")
                || host.ends_with(".hf.co")
        })
}

fn network_error() -> ScanError {
    ScanError::new(
        "classification_download_unavailable",
        "The model download could not continue; retry when the connection is available",
    )
}

#[cfg(test)]
mod tests;
