### Add experimental HTTP transport for Apollo OTLP metrics and traces ([PR #9055](https://github.com/apollographql/router/pull/9055))

The router can now send Apollo OTLP metrics and traces over HTTP (experimental). Enable it with these config values:

- `telemetry.apollo.experimental_otlp_tracing_protocol`
- `telemetry.apollo.experimental_otlp_metrics_protocol`

gRPC remains the preferred transport for Apollo OTLP, but HTTP is available for deployments that can't use gRPC.

By [@bonnici](https://github.com/bonnici) in https://github.com/apollographql/router/pull/9055
