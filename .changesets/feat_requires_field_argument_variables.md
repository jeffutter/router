### Support variable arguments in `@requires` field sets

A `@requires` field set may now bind the arguments of a required field to the annotated field's own arguments, instead of being limited to static literal values. The value a client passes to the annotated field's argument is threaded through to the subgraph that owns the required field at query-planning time.

```graphql
type Product @key(fields: "id") {
  id: ID!
  price(currency: Currency!): Money @external
  localizedPrice(currency: Currency!): Money
    @requires(fields: "price(currency: $currency)")
}
```

Given the operation `{ product { localizedPrice(currency: $userCurrency) } }`, the router now fetches `price(currency: $userCurrency)` from the subgraph that owns `price`, forwarding the client-supplied value rather than a fixed one.

- An omitted argument resolves to its schema default (or `null` for a nullable argument), following GraphQL argument coercion.
- A variable bound to an argument whose type is incompatible with the required field's argument is rejected during composition.

By [@jeffutter](https://github.com/jeffutter) in https://github.com/apollographql/router/pull/9604
