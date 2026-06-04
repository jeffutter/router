use test_log::test;

use super::ServiceDefinition;
use super::assert_composition_errors;
use super::compose_as_fed2_subgraphs;

// =============================================================================
// MERGE VALIDATIONS - Tests for validation during the merge phase
// =============================================================================

#[test]
fn merge_validations_errors_when_a_subgraph_is_invalid() {
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          a: A
        }
        "#,
    };

    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        type A {
          b: Int
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b]);
    assert_composition_errors(
        &result,
        &[(
            "INVALID_GRAPHQL",
            r#"[subgraphA] Error: cannot find type `A` in this document
   ╭─[ subgraphA:3:14 ]
   │
 3 │           a: A
   │              ┬  
   │              ╰── not found in this scope
───╯
"#,
        )],
    );
}

#[test]
fn merge_validations_errors_when_subgraph_has_introspection_reserved_name() {
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          __someQuery: Int
        }
        "#,
    };

    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        type Query {
          aValidOne: Int
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b]);
    assert_composition_errors(
        &result,
        &[(
            "INVALID_GRAPHQL",
            r#"[subgraphA] Error: a field cannot be named `__someQuery` as names starting with two underscores are reserved
   ╭─[ subgraphA:3:11 ]
   │
 3 │           __someQuery: Int
   │           ─────┬─────  
   │                ╰─────── Pick a different name here
───╯
"#,
        )],
    );
}

#[test]
fn merge_validations_errors_when_tag_definition_is_invalid() {
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          a: String
        }

        directive @tag on ENUM_VALUE
        "#,
    };

    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        type A {
          b: Int
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b]);
    assert_composition_errors(
        &result,
        &[(
            "DIRECTIVE_DEFINITION_INVALID",
            r#"[subgraphA] Invalid definition for directive "@tag": missing required argument "name""#,
        )],
    );
}

#[test]
fn merge_validations_reject_subgraph_named_underscore() {
    let subgraph_a = ServiceDefinition {
        name: "_",
        type_defs: r#"
        type Query {
          a: String
        }
        "#,
    };

    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        type A {
          b: Int
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b]);
    assert_composition_errors(
        &result,
        &[(
            "INVALID_SUBGRAPH_NAME",
            "[_] Invalid name _ for a subgraph: this name is reserved",
        )],
    );
}

#[test]
fn merge_validations_reject_if_no_subgraphs_have_query() {
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type A {
          a: Int
        }
        "#,
    };

    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        type B {
          b: Int
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b]);
    assert_composition_errors(
        &result,
        &[(
            "QUERY_ROOT_MISSING",
            "No queries found in any subgraph: a supergraph must have a query root type.",
        )],
    );
}

#[test]
fn merge_validations_reject_type_defined_with_different_kinds() {
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          q: A
        }

        type A {
          a: Int
        }
        "#,
    };

    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        interface A {
          b: Int
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b]);
    assert_composition_errors(
        &result,
        &[(
            "TYPE_KIND_MISMATCH",
            r#"Type "A" has mismatched kind: it is defined as Object Type in subgraph "subgraphA" but Interface Type in subgraph "subgraphB""#,
        )],
    );
}

#[test]
fn merge_validations_errors_if_external_field_not_defined_elsewhere() {
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          q: String
        }
        "#,
    };

    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        interface I {
          f: Int
        }

        type A implements I @key(fields: "k") {
          k: ID!
          f: Int @external
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b]);
    assert_composition_errors(
        &result,
        &[(
            "EXTERNAL_MISSING_ON_BASE",
            r#"Field "A.f" is marked @external on all the subgraphs in which it is listed (subgraph "subgraphB")."#,
        )],
    );
}

#[test]
fn merge_validations_errors_if_mandatory_argument_not_in_all_subgraphs() {
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          q(a: Int!): String @shareable
        }
        "#,
    };

    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        type Query {
          q: String @shareable
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b]);
    assert_composition_errors(
        &result,
        &[(
            "REQUIRED_ARGUMENT_MISSING_IN_SOME_SUBGRAPH",
            r#"Argument "Query.q(a:)" is required in some subgraphs but does not appear in all subgraphs: it is required in subgraph "subgraphA" but does not appear in subgraph "subgraphB""#,
        )],
    );
}

#[test]
fn merge_validations_errors_if_subgraph_required_without_args_but_mandatory_in_supergraph() {
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          t: T
        }

        type T @key(fields: "id") {
          id: ID!
          x(arg: Int): Int @external
          y: Int @requires(fields: "x")
        }
        "#,
    };

    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        type T @key(fields: "id") {
          id: ID!
          x(arg: Int!): Int
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b]);
    assert_composition_errors(
        &result,
        &[(
            "REQUIRES_INVALID_FIELDS",
            r#"[subgraphA] On field "T.y", for @requires(fields: "x"): no value provided for argument "arg" of field "T.x" but a value is mandatory as "arg" is required in subgraph "subgraphB""#,
        )],
    );
}

#[test]
fn merge_validations_errors_if_subgraph_required_with_arg_not_in_supergraph() {
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          t: T
        }

        type T @key(fields: "id") {
          id: ID!
          x(arg: Int): Int @external
          y: Int @requires(fields: "x(arg: 42)")
        }
        "#,
    };

    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        type T @key(fields: "id") {
          id: ID!
          x: Int
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b]);
    assert_composition_errors(
        &result,
        &[(
            "REQUIRES_INVALID_FIELDS",
            r#"[subgraphA] On field "T.y", for @requires(fields: "x(arg: 42)"): cannot provide a value for argument "arg" of field "T.x" as argument "arg" is not defined in subgraph "subgraphB""#,
        )],
    );
}

// Tests for `@requires` referencing a field that takes arguments.
//
// Federation supports static literal argument values in a `@requires` field set. This group also
// covers binding the argument value to a GraphQL variable that names one of the annotated field's
// own arguments (e.g. an end user's currency or locale): at planning time the value the operation
// supplies for that argument is threaded through to the subgraph that owns the required field —
// inlined when it is a literal, or forwarded as a variable resolved at execution time. See:
// https://www.apollographql.com/docs/graphos/schema-design/federated-schemas/entities/contribute-fields#using-requires-with-fields-that-take-arguments

#[test]
fn requires_with_static_argument_value_composes() {
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          t: T
        }

        type T @key(fields: "id") {
          id: ID!
          x(arg: Int): Int @external
          y: Int @requires(fields: "x(arg: 42)")
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, requires_variable_subgraph_b()]);
    assert_composition_succeeds(&result, "a static @requires argument value");
}

/// `subgraphB` shared by the `@requires`-argument tests: it owns the external `x(arg:)` field.
fn requires_variable_subgraph_b() -> ServiceDefinition<'static> {
    ServiceDefinition {
        name: "subgraphB",
        type_defs: REQUIRES_VARIABLE_SUBGRAPH_B,
    }
}

/// Asserts composition succeeded, rendering the composition errors with their codes otherwise.
fn assert_composition_succeeds<T>(
    result: &Result<T, apollo_federation::composition::CompositionFailure>,
    context: &str,
) {
    if let Err(failure) = result {
        let errors: Vec<_> = failure
            .errors
            .iter()
            .map(|error| format!("[{}] {}", error.code().definition().code(), error))
            .collect();
        panic!("composition with {context} should succeed, got: {errors:?}");
    }
}

#[test]
fn requires_with_variable_argument_value_composes() {
    // Sibling of `requires_with_static_argument_value_composes`, but the required field's
    // argument value is a variable bound to the annotated field's own argument (`y(arg:)`) rather
    // than a literal. This test only asserts that such a schema composes; the
    // `..._threads_into_subgraph_fetch` test below proves the value is actually threaded through.
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          t: T
        }

        type T @key(fields: "id") {
          id: ID!
          x(arg: Int): Int @external
          y(arg: Int): Int @requires(fields: "x(arg: $arg)")
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, requires_variable_subgraph_b()]);
    assert_composition_succeeds(&result, "a variable @requires argument value");
}

#[test]
fn requires_with_unbound_variable_argument_value_is_rejected() {
    // `$arg` is not an argument of the annotated field `y`, so it cannot be bound to anything and
    // must be rejected with a clear field-set error (not an internal error).
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          t: T
        }

        type T @key(fields: "id") {
          id: ID!
          x(arg: Int): Int @external
          y: Int @requires(fields: "x(arg: $arg)")
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, requires_variable_subgraph_b()]);
    assert_composition_errors(
        &result,
        &[(
            "REQUIRES_INVALID_FIELDS",
            r#"[subgraphA] On field "T.y", for @requires(fields: "x(arg: $arg)"): variable "$arg" is not defined; a variable in a @requires field set must reference an argument of "T.y""#,
        )],
    );
}

/// Composes the two given subgraphs (named `subgraphA`/`subgraphB`) with the Rust composer, builds
/// a query planner from the result, plans `operation`, and returns the rendered query plan.
///
/// This drives the query planner directly from a Rust-composed supergraph because the rover-backed
/// `planner!` macro can't be used for `@requires` argument variables (JS composition does not
/// accept this syntax).
fn requires_variable_query_plan(
    subgraph_a_type_defs: &str,
    subgraph_b_type_defs: &str,
    operation: &str,
) -> String {
    use apollo_compiler::ExecutableDocument;
    use apollo_federation::Supergraph;
    use apollo_federation::query_plan::query_planner::QueryPlanner;

    let supergraph = compose_as_fed2_subgraphs(&[
        ServiceDefinition {
            name: "subgraphA",
            type_defs: subgraph_a_type_defs,
        },
        ServiceDefinition {
            name: "subgraphB",
            type_defs: subgraph_b_type_defs,
        },
    ])
    .expect("composition should succeed");
    let supergraph_sdl = supergraph.schema().schema().to_string();
    let supergraph =
        Supergraph::new_with_router_specs(&supergraph_sdl).expect("valid supergraph schema");
    let planner = QueryPlanner::new(&supergraph, Default::default()).expect("query planner builds");
    let document = ExecutableDocument::parse_and_validate(
        planner.api_schema().schema(),
        operation,
        "op.graphql",
    )
    .expect("valid operation");
    planner
        .build_query_plan(&document, None, Default::default())
        .expect("query plan is generated")
        .to_string()
}

const REQUIRES_VARIABLE_SUBGRAPH_B: &str = r#"
    type T @key(fields: "id") {
      id: ID!
      x(arg: Int): Int
    }
"#;

// End-to-end query planning: the value the client passes to `t.y(arg:)` is threaded into the
// `x(arg:)` fetch on the subgraph that owns `x` — as a forwarded variable, and (in the literal
// case) inlined.
#[test]
fn requires_with_variable_argument_value_threads_into_subgraph_fetch() {
    let subgraph_a = r#"
        type Query {
          t: T
        }

        type T @key(fields: "id") {
          id: ID!
          x(arg: Int): Int @external
          y(arg: Int): Int @requires(fields: "x(arg: $arg)")
        }
        "#;

    let plan = requires_variable_query_plan(
        subgraph_a,
        REQUIRES_VARIABLE_SUBGRAPH_B,
        "query($v: Int) { t { y(arg: $v) } }",
    );
    assert!(
        plan.contains("x(arg: $v)"),
        "expected the subgraph fetch to request `x(arg: $v)` with the client variable threaded through, but got plan:\n{plan}"
    );

    let literal_plan = requires_variable_query_plan(
        subgraph_a,
        REQUIRES_VARIABLE_SUBGRAPH_B,
        "{ t { y(arg: 7) } }",
    );
    assert!(
        literal_plan.contains("x(arg: 7)"),
        "expected the subgraph fetch to request `x(arg: 7)` with the literal inlined, but got plan:\n{literal_plan}"
    );
}

// When the client omits a nullable argument the variable is bound to, it resolves to `null` per
// GraphQL argument coercion, and `null` is what gets threaded into the required field's fetch.
#[test]
fn requires_with_omitted_nullable_argument_substitutes_null() {
    let subgraph_a = r#"
        type Query {
          t: T
        }

        type T @key(fields: "id") {
          id: ID!
          x(arg: Int): Int @external
          y(arg: Int): Int @requires(fields: "x(arg: $arg)")
        }
        "#;

    let plan =
        requires_variable_query_plan(subgraph_a, REQUIRES_VARIABLE_SUBGRAPH_B, "{ t { y } }");
    assert!(
        plan.contains("x(arg: null)"),
        "expected the omitted nullable argument to resolve to null in the fetch, but got plan:\n{plan}"
    );
}

// When the client omits an argument that has a schema default, the default is what gets threaded
// into the required field's fetch.
#[test]
fn requires_with_omitted_defaulted_argument_substitutes_default() {
    let subgraph_a = r#"
        type Query {
          t: T
        }

        type T @key(fields: "id") {
          id: ID!
          x(arg: Int): Int @external
          y(arg: Int = 5): Int @requires(fields: "x(arg: $arg)")
        }
        "#;

    let plan =
        requires_variable_query_plan(subgraph_a, REQUIRES_VARIABLE_SUBGRAPH_B, "{ t { y } }");
    assert!(
        plan.contains("x(arg: 5)"),
        "expected the argument's schema default (5) to be threaded into the fetch, but got plan:\n{plan}"
    );
}

// The variable can appear nested inside a list-valued argument; substitution recurses into it.
#[test]
fn requires_with_variable_in_list_argument_threads_through() {
    let subgraph_a = r#"
        type Query {
          t: T
        }

        type T @key(fields: "id") {
          id: ID!
          x(args: [Int]): Int @external
          y(arg: Int): Int @requires(fields: "x(args: [$arg])")
        }
        "#;
    let subgraph_b = r#"
        type T @key(fields: "id") {
          id: ID!
          x(args: [Int]): Int
        }
        "#;

    let plan = requires_variable_query_plan(
        subgraph_a,
        subgraph_b,
        "query($v: Int) { t { y(arg: $v) } }",
    );
    assert!(
        plan.contains("x(args: [$v])"),
        "expected the variable to be threaded into the list-valued argument, but got plan:\n{plan}"
    );
}

// A field set may require multiple fields, each binding a distinct field argument.
#[test]
fn requires_with_multiple_variable_arguments_threads_each() {
    let subgraph_a = r#"
        type Query {
          t: T
        }

        type T @key(fields: "id") {
          id: ID!
          x(arg: Int): Int @external
          z(arg: Int): Int @external
          y(a: Int, b: Int): Int @requires(fields: "x(arg: $a) z(arg: $b)")
        }
        "#;
    let subgraph_b = r#"
        type T @key(fields: "id") {
          id: ID!
          x(arg: Int): Int
          z(arg: Int): Int
        }
        "#;

    let plan = requires_variable_query_plan(
        subgraph_a,
        subgraph_b,
        "query($v1: Int, $v2: Int) { t { y(a: $v1, b: $v2) } }",
    );
    assert!(
        plan.contains("x(arg: $v1)") && plan.contains("z(arg: $v2)"),
        "expected both variables threaded into their respective fetches, but got plan:\n{plan}"
    );
}

// A variable whose bound argument type is incompatible with the required field's argument type is
// rejected at composition.
#[test]
fn requires_with_incompatible_variable_type_is_rejected() {
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          t: T
        }

        type T @key(fields: "id") {
          id: ID!
          x(arg: Int): Int @external
          y(arg: String): Int @requires(fields: "x(arg: $arg)")
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, requires_variable_subgraph_b()]);
    assert_composition_errors(
        &result,
        &[(
            "REQUIRES_INVALID_FIELDS",
            r#"[subgraphA] On field "T.y", for @requires(fields: "x(arg: $arg)"): variable "$arg" cannot be used for argument "arg" of field "x": it is bound to argument "arg" of type "String", which is not compatible with the expected type "Int""#,
        )],
    );
}

// `@provides` field sets cannot reference variables; such a reference must fail composition.
#[test]
fn provides_with_variable_argument_is_rejected() {
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          t: T
        }

        type T @key(fields: "id") {
          id: ID!
          u(arg: Int): U @provides(fields: "x(arg: $arg)")
        }

        type U @key(fields: "id") {
          id: ID!
          x(arg: Int): Int @external
        }
        "#,
    };
    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        type U @key(fields: "id") {
          id: ID!
          x(arg: Int): Int
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b]);
    assert!(
        result.is_err(),
        "expected composition to reject a @provides field set referencing a variable"
    );
}

// =============================================================================
// POST-MERGE VALIDATIONS - Tests for validation after the merge phase
// =============================================================================

#[test]
fn post_merge_errors_if_type_does_not_implement_interface_post_merge() {
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          I: [I!]
        }

        interface I {
          a: Int
        }

        type A implements I {
          a: Int
          b: Int
        }
        "#,
    };

    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        interface I {
          b: Int
        }

        type B implements I {
          b: Int
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b]);
    assert_composition_errors(
        &result,
        &[(
            "INTERFACE_FIELD_NO_IMPLEM",
            r#"Interface field "I.a" is declared in subgraph "subgraphA" but type "B", which implements "I" only in subgraph "subgraphB" does not have field "a"."#,
        )],
    );
}

#[test]
fn post_merge_errors_if_type_does_not_implement_interface_on_interface() {
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          I: [I!]
        }

        interface I {
          a: Int
        }

        interface J implements I {
          a: Int
          b: Int
        }

        type A implements I & J {
          a: Int
          b: Int
        }
        "#,
    };

    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        interface J {
          b: Int
        }

        type B implements J {
          b: Int
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b]);
    assert_composition_errors(
        &result,
        &[(
            "INTERFACE_FIELD_NO_IMPLEM",
            r#"Interface field "J.a" is declared in subgraph "subgraphA" but type "B", which implements "J" only in subgraph "subgraphB" does not have field "a"."#,
        )],
    );
}

#[test]
fn requires_fields_validated_in_supergraph_schema() {
    // The @requires fields arguments should be validated against the supergraph schema, not the
    // subgraph schema. Thus, even if subgraph B's @requires fields argument value is invalid in the
    // subgraph schema, it's still valid in supergraph after merging.

    // In subgraph A, type B implements interface I.
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          t: T
        }

        interface I {
          x: String
        }

        type B implements I @key(fields: "x") {
          x: String @shareable
          y: String
        }

        type T @key(fields: "id") {
          id: ID!
          i: I
        }
        "#,
    };

    // In subgraph B, type B does NOT implement interface I,
    // but @requires uses an inline fragment `... on B` within an I-typed field.
    // This should be valid because B implements I in the supergraph.
    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        interface I {
          x: String
        }

        type B @key(fields: "x") {
          x: String @shareable
          y: String @external
        }

        type T @key(fields: "id") {
          id: ID!
          i: I @external
          computed: String @requires(fields: "i { x ... on B { y } }")
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b]);
    result.expect(
        "Expected composition to succeed when type implements interface in another subgraph",
    );
}

// =============================================================================
// MISC VALIDATIONS - Standalone validation tests
// =============================================================================

#[test]
fn misc_not_broken_by_similar_field_argument_signatures() {
    // This test validates the case from https://github.com/apollographql/federation/issues/1100 is fixed.
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          t: T
        }

        type T @shareable {
          a(x: String): Int
          b(x: Int): Int
        }
        "#,
    };

    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        type T @shareable {
          a(x: String): Int
          b(x: Int): Int
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b]);
    let _supergraph = result.expect("Expected composition to succeed");
}

// =============================================================================
// SATISFIABILITY VALIDATIONS - Tests for satisfiability validation
// =============================================================================

#[test]
fn satisfiability_validation_uses_proper_error_code() {
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          a: A
        }

        type A @shareable {
          x: Int
        }
        "#,
    };

    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        type A @shareable {
          x: Int
          y: Int
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b]);
    // This test specifically checks that the error code is SATISFIABILITY_ERROR
    // The exact error message is tested elsewhere
    let errors = result
        .expect_err("Expected composition to fail due to satisfiability")
        .errors;
    let error_codes: Vec<String> = errors
        .iter()
        .map(|e| e.code().definition().code().to_string())
        .collect();
    assert!(
        error_codes
            .iter()
            .any(|msg| msg.contains("SATISFIABILITY_ERROR")),
        "Expected SATISFIABILITY_ERROR but got: {:?}",
        error_codes
    );
}

#[test]
fn satisfiability_validation_handles_indirectly_reachable_keys() {
    // This test ensures that a regression introduced by https://github.com/apollographql/federation/pull/1653
    // is properly fixed. All we want to check is that validation succeeds on this example.
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          t: T
        }

        type T @key(fields: "k1") {
          k1: Int
        }
        "#,
    };

    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        # Note: the ordering of the key happens to matter for this to be a proper reproduction of the
        # issue #1653 created.
        type T @key(fields: "k2") @key(fields: "k1") {
          k1: Int
          k2: Int
        }
        "#,
    };

    let subgraph_c = ServiceDefinition {
        name: "subgraphC",
        type_defs: r#"
        type T @key(fields: "k2") {
          k2: Int
          v: Int
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b, subgraph_c]);
    let _supergraph = result.expect("Expected composition to succeed - satisfiability should pass");
}

#[test]
fn interface_field_no_implem_error_includes_source_locations() {
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          I: [I!]
        }

        interface I {
          a: Int
        }

        type A implements I {
          a: Int
          b: Int
        }
        "#,
    };

    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        interface I {
          b: Int
        }

        type B implements I {
          b: Int
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b]);
    assert_composition_errors(
        &result,
        &[(
            "INTERFACE_FIELD_NO_IMPLEM",
            r#"Interface field "I.a" is declared in subgraph "subgraphA" but type "B", which implements "I" only in subgraph "subgraphB" does not have field "a"."#,
        )],
    );

    let errors = result.expect_err("Expected composition to fail").errors;
    let locs = errors[0].locations();
    assert_eq!(
        locs.len(),
        2,
        "Expected 2 locations (interface field def + implementing type def), got {locs:#?}"
    );
    assert_eq!(locs[0].subgraph, "subgraphA");
    assert_eq!(locs[1].subgraph, "subgraphB");
}

#[test]
fn requires_error_locations_include_requires_directive_and_incompatible_fields() {
    let subgraph_a = ServiceDefinition {
        name: "subgraphA",
        type_defs: r#"
        type Query {
          t: T
        }

        type T @key(fields: "id") {
          id: ID!
          x(arg: Int): Int @external
          y: Int @requires(fields: "x(arg: 42)")
        }
        "#,
    };

    let subgraph_b = ServiceDefinition {
        name: "subgraphB",
        type_defs: r#"
        type T @key(fields: "id") {
          id: ID!
          x: Int
        }
        "#,
    };

    let result = compose_as_fed2_subgraphs(&[subgraph_a, subgraph_b]);
    assert_composition_errors(
        &result,
        &[(
            "REQUIRES_INVALID_FIELDS",
            r#"[subgraphA] On field "T.y", for @requires(fields: "x(arg: 42)"): cannot provide a value for argument "arg" of field "T.x" as argument "arg" is not defined in subgraph "subgraphB""#,
        )],
    );
    let failure = result.unwrap_err();
    let locations = failure.errors[0].locations();
    assert_eq!(
        locations.len(),
        2,
        "Expected 2 locations (the @requires directive and the incompatible field), got {}: {:?}",
        locations.len(),
        locations,
    );
    assert_eq!(locations[0].subgraph, "subgraphA");
    assert_eq!(locations[1].subgraph, "subgraphB");
}
