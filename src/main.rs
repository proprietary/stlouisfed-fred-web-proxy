use std::net::{IpAddr, Ipv4Addr};

use axum::{
    extract::{Query, State},
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

type SharedAppState = std::sync::Arc<std::sync::RwLock<AppState>>;

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
    let app_state: SharedAppState = std::sync::Arc::new(std::sync::RwLock::new(AppState {
        client,
        fred_api_key: cli.fred_api_key,
        realtime_observations_db: RealtimeObservationsDatabase::new(&cli.sqlite_db).await?,
    }));
    app_state
        .write()
        .unwrap()
        .realtime_observations_db
        .create_tables()
        .await?;
    let app = Router::new()
        .route("/v0/observations", get(get_observations_handler))
        .route("/v0/series", get(get_series_handler))
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
    State(app_state_): State<SharedAppState>,
    Query(params): Query<GetSeriesParams>,
) -> Result<Json<FredEconomicDataSeries>, FredApiError> {
    let app_state: &AppState = &app_state_.read().unwrap();
    let series_response = request_series_from_fred(
        app_state.client.clone(),
        &app_state.fred_api_key,
        &params.series_id,
    )
    .await?;
    let series: &FredEconomicDataSeries = series_response.seriess.get(0).ok_or(FredApiError {
        status_code: StatusCode::NOT_FOUND,
        error_message: None,
    })?;
    let maybe_stored_series = app_state
        .realtime_observations_db
        .get_series(&params.series_id)
        .await;
    if let Ok(Some(stored_series)) = maybe_stored_series {
        if stored_series.last_updated < series.last_updated {
            // update stored version
            app_state
                .realtime_observations_db
                .put_series(&series)
                .await
                .map_err(|_| FredApiError {
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                    error_message: Some("database error".into()),
                })?;
        }
    }
    Ok(Json(series.clone()))
}

async fn get_observations_handler(
    Query(params): Query<GetObservationsParams>,
    State(app_state_): State<SharedAppState>,
) -> Result<Json<Vec<RealtimeObservation>>, FredApiError> {
    let app_state: &AppState = &app_state_.read().unwrap();
    let mut observations = std::vec::Vec::<RealtimeObservation>::new();
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
        .map_err(|_| FredApiError::default())?;
    match (cached.len(), cached.get(0), cached.last()) {
        (0, _, _) | (_, None, None) | (_, None, Some(_)) | (_, Some(_), None) => {
            // cache miss
            observations = request_observations_from_fred(
                app_state.client.clone(),
                &app_state.fred_api_key,
                &params.series_id,
                params.observation_start,
                params.observation_end,
                None,
                None,
            )
            .await?;
        }

        // some cached but possibly incomplete
        (_, Some(first_item), Some(last_item)) => {
            let mut is_incomplete: bool = false;

            // check left side
            if let Some(observation_start) = params.observation_start {
                if first_item.date > observation_start {
                    let more = request_observations_from_fred(
                        app_state.client.clone(),
                        &app_state.fred_api_key,
                        &params.series_id,
                        Some(observation_start),
                        Some(first_item.date - chrono::Duration::days(1)),
                        None,
                        None,
                    )
                    .await?;
                    if more.len() > 0 {
                        observations.extend_from_slice(&more);
                        is_incomplete = true;
                    }
                }
            }
            observations.extend_from_slice(&cached);
            // check right side
            if !is_incomplete && params.observation_end.is_some() {
                let observation_end = params.observation_end.unwrap();
                if last_item.date < observation_end {
                    let more = request_observations_from_fred(
                        app_state.client.clone(),
                        &app_state.fred_api_key,
                        &params.series_id,
                        Some(last_item.date + chrono::Duration::days(1)),
                        Some(observation_end),
                        None,
                        None,
                    )
                    .await?;
                    if more.len() > 0 {
                        observations.extend_from_slice(&more);
                        is_incomplete = true;
                    }
                }
            }

            if is_incomplete {
                observations = request_observations_from_fred(
                    app_state.client.clone(),
                    &app_state.fred_api_key,
                    &params.series_id,
                    params.observation_start,
                    params.observation_end,
                    None,
                    None,
                )
                .await?;
            }
        }
    }
    app_state
        .realtime_observations_db
        .put_observations(&params.series_id, &observations)
        .await
        .map_err(|_| FredApiError::default())?;
    Ok(axum::Json(observations))
}
