# FRED web proxy

This is a web server that proxies requests from [St Louis Fed's FRED](https://fred.stlouisfed.org/) (a great resource for free, public domain economics data) with permissive CORS so that web applications can consume it directly.

## Problem this solves

You may have tried to integrate FRED data in your web app frontend. But fetching the FRED API at `https://api.stlouisfed.org/...` from your frontend does not work. This is because (1) FRED does not allow your website's domain in their `Access-Control-Allow-Origin`, and (2) were you to send requests to the FRED API from your frontend, it would be insecure because you would necessarily leak your API key to clients. The obvious solution is to introduce some server-side code that glues together FRED and your client-side application; that's what this is.

This is quick, easy-to-deploy proxy that glues the FRED API to something that your frontend can use.

Also, this may be deployed as a service for anyone on the web to use. If your app is fully client-side, and you need it to consume FRED data, then a public instance of this proxy would solve your problem. 

## Usage

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
$ export FRED_API_KEY=<your api key>
$ ./target/release/stlouisfed-fred-web-proxy
```

```bash
$ curl -s 'http://localhost:9001/v0/observation?series_id=SP500&observation_start=2023-01-01&observation_end=2023-03-01'
```

## API

### `/v0/observations`

This endpoint corresponds to the similar `observations` endpoint, as you can learn more about on [official FRED docs](https://fred.stlouisfed.org/docs/api/fred/series_observations.html).

Available parameters (as query string):
- `series_id`
- `observation_start`
- `observation_end`

Returns an array of dates and values in JSON format.

## License

Apache-2.0