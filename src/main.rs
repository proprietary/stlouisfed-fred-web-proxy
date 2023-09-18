use std::net::{IpAddr, Ipv4Addr};

use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
    Router,
};
use chrono::NaiveDate;
use clap::Parser;
use hyper::StatusCode;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
};

use stlouisfed_fred_web_proxy::{
    entities::{FredResponseObservation, GetObservationsParams, RealtimeObservation},
    local_cache::RealtimeObservationsDatabase,
};

#[derive(Clone)]
struct AppState {
    client: reqwest::Client,
    fred_api_key: String,
    realtime_observations_db: RealtimeObservationsDatabase,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct CommandLineInterface {
    /// Port the HTTP server listens on
    #[arg(short, long, default_value_t = 9001)]
    port: u16,

    /// Path to embedded database which stores previously-fetched FRED data
    #[arg(long, value_name = "FILE", env = "FRED_OBSERVATIONS_DB")]
    sqlite_db: std::path::PathBuf,

    /// Free API key from https://fred.stlouisfed.org
    #[arg(short, long, env = "FRED_API_KEY")]
    fred_api_key: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = CommandLineInterface::parse();
    let client = reqwest::Client::new();
    let port = cli.port;
    let app_state = AppState {
        client: client,
        fred_api_key: cli.fred_api_key,
        realtime_observations_db: RealtimeObservationsDatabase::new(&cli.sqlite_db).await?,
    };
    app_state.realtime_observations_db.create_tables().await?;
    let app = Router::new()
        .route("/v0/observations", get(get_observations_handler))
        .layer(CorsLayer::new().allow_origin(Any))
        .layer(CompressionLayer::new().gzip(true))
        .with_state(app_state);
    let bind_addr: std::net::SocketAddr =
        std::net::SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
    axum::Server::bind(&bind_addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

async fn get_observations_handler(
    Query(params): Query<GetObservationsParams>,
    State(app_state): State<AppState>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut observations = std::vec::Vec::<RealtimeObservation>::new();
    // if user requested realtime/"ALFRED" data, then do not use local cache
    if params.realtime_start.is_some() || params.realtime_end.is_some() {
        // bypass cache
        // because not willing to cache different versions of the same data over and over
        let fresh = request_observations_from_fred(
            &app_state,
            &params.series_id,
            params.observation_start,
            params.observation_end,
            params.realtime_start,
            params.realtime_end,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        return Ok(axum::Json(fresh));
    }
    let cached = app_state
        .realtime_observations_db
        .get_observations(
            &params.series_id,
            params.observation_start,
            params.observation_end,
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    match (cached.len(), cached.get(0), cached.last()) {
        (0, _, _) | (_, None, None) | (_, None, Some(_)) | (_, Some(_), None) => {
            // cache miss
            observations = request_observations_from_fred(
                &app_state,
                &params.series_id,
                params.observation_start,
                params.observation_end,
                None,
                None,
            )
            .await
            .map_err(|e| match e.status() {
                Some(status) => StatusCode::from(status),
                None => StatusCode::SERVICE_UNAVAILABLE,
            })?;
        }

        // some cached but possibly incomplete
        (_, Some(first_item), Some(last_item)) => {
            // check left side
            if let Some(observation_start) = params.observation_start {
                if first_item.date > observation_start {
                    let more = request_observations_from_fred(
                        &app_state,
                        &params.series_id,
                        Some(observation_start),
                        Some(first_item.date - chrono::Duration::days(1)),
                        None,
                        None,
                    )
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    if more.len() > 0 {
                        observations.extend_from_slice(&more);
                    }
                }
            }
            observations.extend_from_slice(&cached);
            // check right side
            if let Some(observation_end) = params.observation_end {
                if last_item.date < observation_end {
                    let more = request_observations_from_fred(
                        &app_state,
                        &params.series_id,
                        Some(last_item.date + chrono::Duration::days(1)),
                        Some(observation_end),
                        None,
                        None,
                    )
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    observations.extend_from_slice(&more);
                }
            }
        }
    }
    app_state
        .realtime_observations_db
        .put_observations(&params.series_id, &observations)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    return Ok(axum::Json(observations));
}

async fn request_observations_from_fred(
    app_state: &AppState,
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
                .append_pair("api_key", &app_state.fred_api_key)
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
        let req = app_state.client.get(url).send().await;
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
