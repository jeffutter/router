use std::num::NonZero;

use apollo_compiler::ExecutableDocument;
use apollo_federation::query_plan::query_planner::{
    QueryPlanIncrementalDeliveryConfig, QueryPlanOptions, QueryPlanner, QueryPlannerConfig,
    QueryPlannerDebugConfig,
};

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <schema_file> <query_file> <query_name>", args[0]);
        std::process::exit(1);
    }

    let schema_content = std::fs::read_to_string(&args[1]).expect("Failed to read schema");
    let query_content = std::fs::read_to_string(&args[2]).expect("Failed to read query");
    let query_name = &args[3];

    let supergraph =
        apollo_federation::Supergraph::new(&schema_content).expect("Couldn't load supergraph");

    let config = QueryPlannerConfig {
        incremental_delivery: QueryPlanIncrementalDeliveryConfig { enable_defer: true },
        subgraph_graphql_validation: false,
        generate_query_fragments: true,
        debug: QueryPlannerDebugConfig {
            max_evaluated_plans: NonZero::new(10000).unwrap(),
            paths_limit: None,
        },
        type_conditioned_fetching: false,
    };

    let planner = QueryPlanner::new(&supergraph, config).expect("Could not create Planner");

    let query_doc = ExecutableDocument::parse_and_validate(
        planner.api_schema().schema(),
        query_content,
        &args[1],
    )
    .expect("Invalid Query");

    let plan = planner
        .build_query_plan(
            &query_doc,
            Some(apollo_compiler::Name::new(query_name).unwrap()),
            QueryPlanOptions {
                override_conditions: vec![],
            },
        )
        .expect("Unable to plan query");

    let plan_str = if let Some(ref root_node) = plan.node {
        match root_node {
            apollo_federation::query_plan::TopLevelPlanNode::Subscription(_) => {
                todo!()
            }
            apollo_federation::query_plan::TopLevelPlanNode::Fetch(ref f) => {
                let p = f.operation_document.serialize();
                format!("{}", p)
            }
            apollo_federation::query_plan::TopLevelPlanNode::Sequence(_) => todo!(),
            apollo_federation::query_plan::TopLevelPlanNode::Parallel(_) => todo!(),
            apollo_federation::query_plan::TopLevelPlanNode::Flatten(_) => todo!(),
            apollo_federation::query_plan::TopLevelPlanNode::Defer(_) => todo!(),
            apollo_federation::query_plan::TopLevelPlanNode::Condition(_) => todo!(),
        }
    } else {
        unimplemented!()
    };
    // let plan_str = format!("{}", plan);
    //
    print!(
        "{}",
        graphql_parser::minify_query(plan_str).expect("Couldn't minify")
    );
    // print!("{}", plan)
}
