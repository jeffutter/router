### Recognize 204 (No Content) responses without `Content-Length` header in connectors ([PR #9141](https://github.com/apollographql/router/pull/9141))

Connectors now correctly handle HTTP 204 (No Content) responses from spec-compliant servers that don't include a `Content-Length` header.

Previously, empty body detection relied on the presence of a `Content-Length: 0` header. Because the HTTP spec explicitly forbids including this header in 204 responses, connectors would fail to recognize empty bodies from compliant servers. The fix checks `body.is_empty()` directly, with `Content-Length: 0` kept as a fallback for non-compliant servers.

By [@apollo-mateuswgoettems](https://github.com/apollo-mateuswgoettems) in https://github.com/apollographql/router/pull/9141