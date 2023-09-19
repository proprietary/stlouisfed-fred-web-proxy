use std::net::{IpAddr, Ipv4Addr};

use axum::{
    extract::{Query, State},
    response::Redirect,
    routing::get,
    Json, Router,
};
use clap::Parser;
use hyper::StatusCode;
use tower_http::{
    compression::CompressionLayer,
    cors::{Any, CorsLayer},
};

use stlouisfed_fred_web_proxy::{
    entities::{
        FredEconomicDataSeries, GetObservationsParams, GetSeriesParams, RealtimeObservation,
    },
    fred::{request_observations_from_fred, request_series_from_fred, FredApiError},
    local_cache::RealtimeObservationsDatabase,
};

#[derive(Clone)]
struct AppState {
    client: reqwest::Client,
    fred_api_key: String,
    realtime_observations_db: RealtimeObservationsDatabase,
}

// type SharedAppState = std::sync::Arc<std::sync::RwLock<AppState>>;

// impl Default for AppState {
//     fn default() -> Self {
//         AppState {
//             client: Default::default(),
//             fred_api_key: Default::default(),
//             realtime_observations_db: None,
//         }
//     }
// }

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
        client,
        fred_api_key: cli.fred_api_key,
        realtime_observations_db: RealtimeObservationsDatabase::new(&cli.sqlite_db).await?,
    };
    app_state.realtime_observations_db.create_tables().await?;
    let app = Router::new()
        .route("/v0/observations", get(get_observations_handler))
        .route("/v0/series", get(get_series_handler))
        .route(
            "/",
            get(Redirect::temporary(
                "https://github.com/proprietary/stlouisfed-fred-web-proxy",
            )),
        )
        .layer(CorsLayer::new().allow_origin(Any))
        .layer(CompressionLayer::new().gzip(true))
        .with_state(app_state.clone());
    let bind_addr: std::net::SocketAddr =
        std::net::SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
    axum::Server::bind(&bind_addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
    Ok(())
}

async fn get_series_handler(
    State(app_state): State<AppState>,
    Query(params): Query<GetSeriesParams>,
) -> Result<Json<FredEconomicDataSeries>, FredApiError> {
    let series_response = request_series_from_fred(
        app_state.client.clone(),
        &app_state.fred_api_key,
        &params.series_id,
    )
    .await?;
    let series: FredEconomicDataSeries = series_response
        .seriess
        .get(0)
        .ok_or(FredApiError {
            status_code: StatusCode::NOT_FOUND,
            error_message: None,
        })?
        .clone();
    let maybe_stored_series = app_state
        .realtime_observations_db
        .get_series(&params.series_id)
        .await
        .map_err(|_| FredApiError::default())?;
    match maybe_stored_series {
        None => {
            app_state
                .realtime_observations_db
                .put_series(&series.clone())
                .await
                .map_err(|_| FredApiError::default())?;
        }
        Some(stored_series) => {
            if stored_series.last_updated < series.last_updated {
                app_state
                    .realtime_observations_db
                    .put_series(&series.clone())
                    .await
                    .map_err(|_| FredApiError::default())?;
            }
        }
    }
    Ok(Json(series))
}

async fn get_observations_handler(
    State(app_state): State<AppState>,
    Query(params): Query<GetObservationsParams>,
) -> Result<Json<Vec<RealtimeObservation>>, FredApiError> {
    // if user requested realtime/"ALFRED" data, then do not use local cache
    if params.realtime_start.is_some() || params.realtime_end.is_some() {
        // bypass cache
        // because not willing to cache different versions of the same data over and over
        let fresh = request_observations_from_fred(
            app_state.client.clone(),
            &app_state.fred_api_key,
            &params.series_id,
            params.observation_start,
            params.observation_end,
            params.realtime_start,
            params.realtime_end,
        )
        .await?;
        return Ok(Json(fresh));
    }
    let cached = app_state
        .realtime_observations_db
        .get_observations(
            &params.series_id,
            params.observation_start,
            params.observation_end,
        )
        .await
        .map_err(|_| FredApiError::default())?;
    // Check if the cache hit by only checking the `observation_end` boundary.
    // No need to check the beginning. Assume that if the series is present in the database,
    // it has all historical observations available.
    if !cached.is_empty()
        && params.observation_end.is_some()
        && params.observation_end.unwrap() <= cached.last().unwrap().date
    {
        return Ok(Json(cached));
    }
    // Cache miss--so go out to the FRED API to get the requested observations.
    let fresh_observations = request_observations_from_fred(
        app_state.client.clone(),
        &app_state.fred_api_key,
        &params.series_id,
        // only request after the time period we already have stored
        cached
            .last()
            .map(|item| item.date + chrono::Duration::days(1)),
        params.observation_end,
        None,
        None,
    )
    .await?;
    // Update database with externally-sourced observations.
    app_state
        .realtime_observations_db
        .put_observations(&params.series_id, &fresh_observations)
        .await
        .map_err(|_| FredApiError::default())?;
    let mut observations = cached;
    observations.extend_from_slice(&fresh_observations);
    Ok(axum::Json(observations))
}
