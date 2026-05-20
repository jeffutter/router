### Return a single JSON response for unsupported defer-with-batch queries ([PR #9311](https://github.com/apollographql/router/pull/9311))

Batched queries that use `@defer` are not supported by the router.  Previously these requests produced a malformed multipart response; they now return a single JSON response with errors that explicitly indicates the lack of support.

By [@rohan-b99](https://github.com/rohan-b99) in https://github.com/apollographql/router/pull/9311
