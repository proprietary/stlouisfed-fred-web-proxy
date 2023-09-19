# FRED web proxy

This is a web server that proxies requests from [St Louis Fed's FRED](https://fred.stlouisfed.org/) (a great resource for free, public domain economics data) with permissive CORS so that web applications can consume it directly.

## Problem this solves

You may have tried to integrate FRED data in your web app frontend. But fetching the FRED API at `https://api.stlouisfed.org/...` from your frontend does not work. This is because (1) FRED does not allow your website's domain in their `Access-Control-Allow-Origin`, and (2) were you to send requests to the FRED API from your frontend, it would be insecure because you would necessarily leak your API key to clients. The obvious solution is to introduce some server-side code that glues together FRED and your client-side application; that's what this is.

This is quick, easy-to-deploy proxy that glues the FRED API to something that your frontend can use.

Also, this may be deployed as a service for anyone on the web to use. If your app is fully client-side, and you need it to consume FRED data, then a public instance of this proxy would solve your problem. 

## Live Demo

This API is live at `https://fred.libhack.so/`. It is free and open to use right now.

Example:

```bash
# Get 2 specific days of the S&P500 series
% curl 'https://fred.libhack.so/v0/observations?series_id=SP500&observation_start=2023-09-14&observation_end=2023-09-15'
[{"date":"2023-09-14","value":"4505.1"},{"date":"2023-09-15","value":"4450.32"}]
```

## API

### `/v0/observations`

This endpoint corresponds to the similar `observations` endpoint, as you can learn more about on [official FRED docs](https://fred.stlouisfed.org/docs/api/fred/series_observations.html). `series_id` can be most easily found by finding a FRED page and looking at the end of the URL. For example, the `series_id` of `https://fred.stlouisfed.org/series/WLODLL` is `WLODLL`.

Available parameters (as query string parameters):
- `series_id`
- `observation_start`
- `observation_end`

Returns an array of dates and values in JSON format.

### `/v0/series`

This is metadata about an economic series. It forwards the result from FRED's `series` endpoint ([official FRED docs](https://fred.stlouisfed.org/docs/api/fred/series.html)).

Available parameters (as query string parameters):
- `series_id`


## Usage

The following instructions are relevant if you want to run this service yourself.

### Installation

Prerequisites:
- Install the [Rust toolchain](https://rustup.rs/)
- Create a [free API key](https://fred.stlouisfed.org/docs/api/api_key.html) on FRED

```bash
$ git clone https://github.com/proprietary/stlouisfed-fred-web-proxy.git
$ cd stlouisfed-fred-web-proxy
$ cargo build --release
```

### Running

Example:

```bash
$ ./target/release/stlouisfed-fred-web-proxy \
  --sqlite-db cache.db \
  --fred-api-key <your-api-key>
$ # You may also set these configuration variables through environment variables like so:
$ # export FRED_OBSERVATIONS_DB=<path to a file which caches data locally>
$ # export FRED_API_KEY=<your api key>

```

```bash
$ curl -s 'http://localhost:9001/v0/observation?series_id=SP500&observation_start=2023-01-01&observation_end=2023-03-01'
```

## Open source

This software is provided "as is", without warranty of any kind.

This software retrieves and stores data from St. Louis Fed's FRED economics data service. All the data from FRED is in the public domain and therefore free to store, retrieve and redistribute.

## License

Apache-2.0