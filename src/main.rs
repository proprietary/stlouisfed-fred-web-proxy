use std::net::{IpAddr, Ipv4Addr};

use chrono::{DateTime, NaiveDate, Utc};

use axum::{
    extract::{Query, State},
    http,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tower_http::cors::{Any, CorsLayer};

use stlouisfed_fred_web_proxy::{entities, local_cache::RealtimeObservationsDatabase};
use stlouisfed_fred_web_proxy::{local_cache::FREDDatabase, yyyy_mm_dd_date_format};

#[derive(Default, Clone)]
struct AppState {
    client: reqwest::Client,
    fred_api_key: String,
    realtime_observations_db: RealtimeObservationsDatabase,
}

#[tokio::main]
async fn main() {
    let client = reqwest::Client::new();
    let api_key = std::env::var("FRED_API_KEY").expect("Missing FRED_API_KEY env var");
    // let port: u16 = std::env::var("SERVER_PORT")
    //     .expect("Missing SERVER_PORT env var")
    //     .parse()
    //     .expect("Environment variable SERVER_PORT is not an integer");
    let port: u16 = std::env::var("SERVER_PORT")
        .unwrap_or("9001".to_string())
        .parse()
        .unwrap();
    let sqlite_db = std::path::PathBuf::from(
        std::env::var("FRED_OBSERVATIONS_DB").expect("Missing FRED_OBSERVATIONS_DB env var"),
    );
    if !sqlite_db.exists() {
        panic!("Provided sqlite DB path does not exist");
    }
    let app_state = AppState {
        client: client,
        fred_api_key: api_key,
        realtime_observations_db: RealtimeObservationsDatabase::new(&sqlite_db),
    };
    app_state.realtime_observations_db.create_tables().unwrap();
    let app = Router::new()
        .route("/v0/observations", get(get_observations_handler))
        .layer(CorsLayer::new().allow_origin(Any))
        .with_state(app_state);
    let bind_addr: std::net::SocketAddr =
        std::net::SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
    axum::Server::bind(&bind_addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn get_observations_handler(
    Query(params): Query<GetObservationsParams>,
    State(app_state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut observations = std::vec::Vec::<entities::RealtimeObservation>::new();
    let cached = app_state
        .realtime_observations_db
        .get_observations(
            &params.series_id,
            Some(params.observation_start),
            Some(params.observation_end),
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match (cached.get(0), cached.iter().last()) {
        (Some(first_item), Some(last_item)) => {
            if first_item.date <= params.observation_start
                && last_item.date >= params.observation_end
            {
                cached.iter().for_each(|os| {
                    if os.date >= params.observation_start && os.date <= params.observation_end {
                        observations.push(os.clone());
                    }
                });
                return Ok(axum::Json(observations));
            }
        }
        (_, _) => {
            // cache miss
        }
    }
    let mut offset: usize = 0usize;
    const LIMIT: usize = 10_000;
    loop {
        let mut url =
            reqwest::Url::parse("https://api.stlouisfed.org/fred/series/observations").unwrap();

        {
            let mut pairs = (&mut url).query_pairs_mut();
            pairs
                .append_pair("api_key", &app_state.fred_api_key)
                .append_pair("file_type", "json")
                .append_pair("limit", &LIMIT.to_string())
                .append_pair("series_id", &params.series_id)
                .append_pair("observation_start", &params.observation_start.to_string())
                .append_pair("observation_end", &params.observation_end.to_string());
            if offset > 0 {
                pairs.append_pair("offset", &offset.to_string());
            }
            pairs.finish();
        }
        let req = app_state.client.get(url).send().await;
        let output = match req {
            Ok(res) => res.json::<FredResponseObservation>().await.unwrap(),
            Err(err) => {
                if let Some(status) = err.status() {
                    // forward status code from FRED's API
                    return Err(status);
                }
                return Err(StatusCode::SERVICE_UNAVAILABLE);
            }
        };
        output.observations.iter().for_each(|os| {
            observations.push(entities::RealtimeObservation {
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
    app_state
        .realtime_observations_db
        .put_observations(&params.series_id, &observations)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    return Ok(axum::Json(observations));
}

#[derive(Debug, Deserialize)]
struct GetObservationsParams {
    series_id: String,

    #[serde(with = "yyyy_mm_dd_date_format")]
    observation_start: NaiveDate,

    #[serde(with = "yyyy_mm_dd_date_format")]
    observation_end: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ObservationItem {
    #[serde(with = "yyyy_mm_dd_date_format")]
    date: NaiveDate,

    value: String,

    #[serde(with = "yyyy_mm_dd_date_format")]
    realtime_start: NaiveDate,

    #[serde(with = "yyyy_mm_dd_date_format")]
    realtime_end: NaiveDate,
}

#[derive(Default, Debug, Deserialize)]
struct FredResponseObservation {
    #[serde(with = "yyyy_mm_dd_date_format")]
    realtime_start: NaiveDate,

    #[serde(with = "yyyy_mm_dd_date_format")]
    realtime_end: NaiveDate,

    count: usize,

    offset: usize,

    limit: usize,

    observations: std::vec::Vec<ObservationItem>,
}
