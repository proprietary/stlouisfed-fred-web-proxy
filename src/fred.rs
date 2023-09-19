use chrono::NaiveDate;
use hyper::StatusCode;

use crate::entities::{
    FredResponseObservation, FredResponseSeries, FredResponseSeriesWithError, RealtimeObservation,
};

pub async fn request_observations_from_fred(
    client: reqwest::Client,
    fred_api_key: &str,
    series_id: &str,
    observation_start: Option<NaiveDate>,
    observation_end: Option<NaiveDate>,
    realtime_start: Option<NaiveDate>,
    realtime_end: Option<NaiveDate>,
) -> Result<Vec<RealtimeObservation>, reqwest::Error> {
    let mut observations = Vec::<RealtimeObservation>::new();
    let mut offset: usize = 0usize;
    const LIMIT: usize = 10_000;
    const FORMAT: &'static str = "%Y-%m-%d";
    loop {
        let mut url =
            reqwest::Url::parse("https://api.stlouisfed.org/fred/series/observations").unwrap();

        {
            let mut pairs = (&mut url).query_pairs_mut();
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
        let req = client.get(url).send().await;
        let output = req?.json::<FredResponseObservation>().await?;
        output.observations.iter().for_each(|os| {
            observations.push(RealtimeObservation {
                date: os.date,
                value: os.value.clone(),
            });
        });
        if output.observations.len() >= output.limit {
            offset += output.observations.len();
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
) -> Result<FredResponseSeries, StatusCode> {
    let url = reqwest::Url::parse_with_params(
        "https://api.stlouisfed.org/fred/series",
        &[
            ("api_key", fred_api_key),
            ("file_type", &"json".to_string()),
            ("series_id", &series_id.to_string()),
        ][..],
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let output = client
        .get(url)
        .send()
        .await
        .map_err(|e| StatusCode::from(e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)))?
        .json::<FredResponseSeriesWithError>()
        .await
        .map_err(|e| StatusCode::from(e.status().unwrap_or(StatusCode::INTERNAL_SERVER_ERROR)))?;
    match output {
        FredResponseSeriesWithError::FredResponseError(e) => {
            Err(StatusCode::from_u16(e.error_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR))
        }
        FredResponseSeriesWithError::FredResponseSeries(s) => Ok(s),
    }
}
