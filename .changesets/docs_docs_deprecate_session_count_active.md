### Deprecate `apollo.router.session.count.active` in favor of `http.server.active_requests` ([PR #9069](https://github.com/apollographql/router/pull/9069))

The Standard Instruments reference now marks `apollo.router.session.count.active` as deprecated and directs users to [`http.server.active_requests`](https://www.apollographql.com/docs/graphos/routing/observability/router-telemetry-otel/enabling-telemetry/instruments#opentelemetry-standard-instruments) instead, which follows OpenTelemetry semantic conventions. The metric remains in the router for backward compatibility but might be removed in a future release.

By [@mabuyo](https://github.com/mabuyo) in https://github.com/apollographql/router/pull/9069