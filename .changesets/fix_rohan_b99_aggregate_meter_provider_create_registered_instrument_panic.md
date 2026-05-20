### Avoid spurious "meter provider after shutdown" error during router shutdown ([PR #9248](https://github.com/apollographql/router/pull/9248))

The router no longer emits the spurious `cannot use meter provider after shutdown` error during shutdown.  The metrics aggregation layer now returns a noop instrument in that path instead of panicking.

By [@rohan-b99](https://github.com/rohan-b99) in https://github.com/apollographql/router/pull/9248
