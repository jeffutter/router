use apollo_router::_private::QueryPlanResult;
use apollo_router::query_planner::PlanNode::Fetch;
use router_bridge::planner::{
    IncrementalDeliverySupport, PlanOptions, Planner, QueryPlannerConfig, QueryPlannerDebugConfig,
};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <schema_file> <query_file>", args[0]);
        std::process::exit(1);
    }

    let schema_content = std::fs::read_to_string(&args[1]).expect("Failed to read schema");
    let query_content = std::fs::read_to_string(&args[2]).expect("Failed to read query");

    let config = QueryPlannerConfig {
        incremental_delivery: Some(IncrementalDeliverySupport {
            enable_defer: Some(true),
        }),
        graphql_validation: false,
        reuse_query_fragments: Some(true),
        generate_query_fragments: Some(false),
        debug: Some(QueryPlannerDebugConfig {
            bypass_planner_for_single_subgraph: None,
            max_evaluated_plans: Some(10000),
            paths_limit: None,
        }),
        type_conditioned_fetching: false,
    };

    let planner: Planner<QueryPlanResult> = Planner::new(schema_content, config)
        .await
        .expect("Could not create Planner");

    let plan_opts = PlanOptions {
        override_conditions: vec![],
    };

    let plan = planner
        .plan(query_content, Some(String::from("")), plan_opts)
        .await
        .expect("Unable to plan query");

    let plan = plan.data.unwrap().query_plan;

    if let Some(root_node) = plan.node {
        match *root_node {
            Fetch(ref f) => {
                let x = f.operation.as_serialized();
                println!("{}", x);
            }
            _ => unimplemented!(),
        }
    }
}
