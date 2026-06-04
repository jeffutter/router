//! Execution-level coverage for `@requires` field sets whose required field's argument is bound to
//! a variable naming the annotated field's own argument (e.g. `price(currency: $currency)`).
//!
//! The supergraph is composed in-test with the Rust composer (the rover-backed fixtures can't be
//! used because JS composition does not accept this syntax yet), then driven through the router
//! with subgraph services replaced by a capturing stub, to confirm the client-supplied value is
//! actually forwarded to the subgraph that owns the required field.

use std::sync::Arc;
use std::sync::Mutex;

use apollo_federation::composition::CompositionOptions;
use apollo_federation::composition::compose;
use apollo_federation::subgraph::typestate::Subgraph;
use apollo_router::TestHarness;
use apollo_router::services::subgraph;
use apollo_router::services::supergraph;
use serde_json::json;
use tower::ServiceExt;

const SUBGRAPH_A: &str = r#"
    type Query {
      t: T
    }

    type T @key(fields: "id") {
      id: ID!
      x(arg: Int): Int @external
      y(arg: Int): Int @requires(fields: "x(arg: $arg)")
    }
"#;

const SUBGRAPH_B: &str = r#"
    type T @key(fields: "id") {
      id: ID!
      x(arg: Int): Int
    }
"#;

fn compose_supergraph() -> String {
    let subgraph_a = Subgraph::parse("subgraphA", "http://subgraphA", SUBGRAPH_A)
        .unwrap()
        .into_fed2_test_subgraph(true)
        .unwrap();
    let subgraph_b = Subgraph::parse("subgraphB", "http://subgraphB", SUBGRAPH_B)
        .unwrap()
        .into_fed2_test_subgraph(true)
        .unwrap();
    let supergraph = compose(vec![subgraph_a, subgraph_b], CompositionOptions::default())
        .expect("composition should succeed");
    supergraph.schema().schema().to_string()
}

/// A captured subgraph request: (subgraph name, operation string, variables).
type CapturedRequest = (String, String, serde_json_bytes::Value);

/// Plans and executes `operation` against the composed supergraph with every subgraph replaced by a
/// stub that records the request it receives and returns canned data so execution can proceed.
/// Returns the request received by `subgraphB` (the owner of the external `x` field).
async fn captured_subgraph_b_request(operation: &str, variable: Option<i32>) -> CapturedRequest {
    let schema = compose_supergraph();
    let captured: Arc<Mutex<Vec<CapturedRequest>>> = Arc::new(Mutex::new(Vec::new()));
    let captured_for_hook = captured.clone();

    let harness = TestHarness::builder()
        .configuration_json(json!({"include_subgraph_errors": {"all": true}}))
        .unwrap()
        .schema(&schema)
        .subgraph_hook(move |name, _default| {
            let captured = captured_for_hook.clone();
            let name = name.to_string();
            tower::service_fn(move |request: subgraph::Request| {
                let captured = captured.clone();
                let name = name.clone();
                async move {
                    let body = request.subgraph_request.body();
                    let query = body.query.clone().unwrap_or_default();
                    let variables = serde_json_bytes::Value::Object(body.variables.clone());
                    captured
                        .lock()
                        .unwrap()
                        .push((name.clone(), query.clone(), variables));

                    // Return just enough data for each fetch so execution proceeds to the next.
                    let data = if name == "subgraphA" && !query.contains("_entities") {
                        serde_json_bytes::json!({ "t": { "__typename": "T", "id": "1" } })
                    } else if name == "subgraphB" {
                        serde_json_bytes::json!({ "_entities": [ { "x": 100 } ] })
                    } else {
                        serde_json_bytes::json!({ "_entities": [ { "y": 200 } ] })
                    };

                    Ok::<_, tower::BoxError>(
                        subgraph::Response::fake_builder()
                            .data(data)
                            .context(request.context.clone())
                            .subgraph_name(name)
                            .build(),
                    )
                }
            })
            .boxed()
        })
        .build_supergraph()
        .await
        .unwrap();

    let mut request = supergraph::Request::fake_builder().query(operation);
    if let Some(value) = variable {
        request = request.variable("v", value);
    }
    let response = harness
        .oneshot(request.build().unwrap())
        .await
        .unwrap()
        .next_response()
        .await
        .unwrap();
    assert!(
        response.errors.is_empty(),
        "unexpected execution errors: {:?}",
        response.errors
    );

    let captured = captured.lock().unwrap();
    captured
        .iter()
        .find(|(name, _, _)| name == "subgraphB")
        .cloned()
        .expect("subgraphB should have been queried to resolve the required `x` field")
}

#[tokio::test(flavor = "multi_thread")]
async fn forwards_client_variable_to_owning_subgraph() {
    let (_, operation, variables) =
        captured_subgraph_b_request("query($v: Int) { t { y(arg: $v) } }", Some(7)).await;

    assert!(
        operation.replace(' ', "").contains("x(arg:$v)"),
        "subgraphB operation should request `x` with the threaded variable, got: {operation}"
    );
    assert_eq!(
        variables.as_object().and_then(|object| object.get("v")),
        Some(&serde_json_bytes::Value::from(7)),
        "subgraphB should have received the client-supplied value for the threaded variable; variables were {variables}"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn inlines_literal_argument_for_owning_subgraph() {
    let (_, operation, _) = captured_subgraph_b_request("{ t { y(arg: 7) } }", None).await;

    assert!(
        operation.replace(' ', "").contains("x(arg:7)"),
        "subgraphB operation should request `x` with the literal argument inlined, got: {operation}"
    );
}
