use chrono::NaiveDate;
use hyper::StatusCode;

use axum::{
    response::{IntoResponse, Response},
    Json,
};

use crate::entities::{
    FredApiResponse, FredResponseError, FredResponseObservation, FredResponseSeries,
    RealtimeObservation,
};

#[derive(Debug)]
pub struct FredApiError {
    pub status_code: StatusCode,
    pub error_message: Option<String>,
}

impl std::error::Error for FredApiError {}

impl Default for FredApiError {
    fn default() -> Self {
        FredApiError {
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
            error_message: None,
        }
    }
}

impl std::fmt::Display for FredApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        if let Some(ref msg) = self.error_message {
            f.write_str(&format!(
                "FRED API errored with status code {}: {}",
                self.status_code, msg
            ))
        } else {
            f.write_str(&format!(
                "FRED API errored with status code {}",
                self.status_code
            ))
        }
    }
}

impl From<reqwest::Error> for FredApiError {
    fn from(value: reqwest::Error) -> Self {
        FredApiError {
            status_code: value.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            error_message: Some(value.to_string()),
        }
    }
}

impl<T> From<FredApiResponse<T>> for Result<T, FredApiError> {
    fn from(value: FredApiResponse<T>) -> Self {
        match value {
            FredApiResponse::ErrorMessage(e) => Err(FredApiError {
                error_message: Some(e.error_message),
                status_code: StatusCode::from_u16(e.error_code)
                    .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            }),
            FredApiResponse::Payload(response) => Ok(response),
        }
    }
}

impl From<FredResponseError> for FredApiError {
    fn from(value: FredResponseError) -> Self {
        FredApiError {
            status_code: StatusCode::from_u16(value.error_code)
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
            error_message: Some(value.error_message),
        }
    }
}

impl IntoResponse for FredApiError {
    fn into_response(self) -> Response {
        (
            self.status_code,
            Json(self.error_message.unwrap_or_default()),
        )
            .into_response()
    }
}

pub async fn request_observations_from_fred(
    client: reqwest::Client,
    fred_api_key: &str,
    series_id: &str,
    observation_start: Option<NaiveDate>,
    observation_end: Option<NaiveDate>,
    realtime_start: Option<NaiveDate>,
    realtime_end: Option<NaiveDate>,
) -> Result<Vec<RealtimeObservation>, FredApiError> {
    let mut observations = Vec::<RealtimeObservation>::new();
    let mut offset: usize = 0usize;
    const LIMIT: usize = 10_000;
    const FORMAT: &str = "%Y-%m-%d";
    loop {
        let mut url =
            reqwest::Url::parse("https://api.stlouisfed.org/fred/series/observations").unwrap();

        {
            let mut pairs = url.query_pairs_mut();
            pairs
                .append_pair("api_key", fred_api_key)
                .append_pair("file_type", "json")
                .append_pair("limit", &LIMIT.to_string())
                .append_pair("sort_order", "asc")
                .append_pair("series_id", series_id);
            if let Some(observation_start) = observation_start {
                pairs.append_pair(
                    "observation_start",
                    &observation_start.format(FORMAT).to_string(),
                );
            }
            if let Some(observation_end) = observation_end {
                pairs.append_pair(
                    "observation_end",
                    &observation_end.format(FORMAT).to_string(),
                );
            }
            if let Some(realtime_start) = realtime_start {
                pairs.append_pair("realtime_start", &realtime_start.format(FORMAT).to_string());
            }
            if let Some(realtime_end) = realtime_end {
                pairs.append_pair("realtime_end", &realtime_end.format(FORMAT).to_string());
            }
            if offset > 0 {
                pairs.append_pair("offset", &offset.to_string());
            }
            pairs.finish();
        }
        let fred_response_: Result<FredResponseObservation, FredApiError> = client
            .get(url)
            .send()
            .await?
            .json::<FredApiResponse<FredResponseObservation>>()
            .await?
            .into();
        let fred_response = fred_response_?;
        fred_response.observations.iter().for_each(|os| {
            observations.push(RealtimeObservation {
                date: os.date,
                value: os.value.clone(),
            });
        });
        if fred_response.observations.len() >= fred_response.limit {
            offset += fred_response.observations.len();
        } else {
            break;
        }
    }
    Ok(observations)
}

/// Get an economic data series (really, just the metadata).
/// See: https://fred.stlouisfed.org/docs/api/fred/series.html
pub async fn request_series_from_fred(
    client: reqwest::Client,
    fred_api_key: &str,
    series_id: &str,
) -> Result<FredResponseSeries, FredApiError> {
    let url = reqwest::Url::parse_with_params(
        "https://api.stlouisfed.org/fred/series",
        &[
            ("api_key", fred_api_key),
            ("file_type", "json"),
            ("series_id", series_id),
        ][..],
    )
    .map_err(|_| FredApiError::default())?;
    let output: Result<FredResponseSeries, FredApiError> = client
        .get(url)
        .send()
        .await?
        .json::<FredApiResponse<FredResponseSeries>>()
        .await?
        .into();
    output
}
