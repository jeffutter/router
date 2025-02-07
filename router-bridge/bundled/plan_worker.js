"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
function memoryUsage() {
    return Deno.core.ops.op_runtime_memory_usage();
}
let logFunction;
var PlannerEventKind;
(function (PlannerEventKind) {
    PlannerEventKind["UpdateSchema"] = "UpdateSchema";
    PlannerEventKind["Plan"] = "Plan";
    PlannerEventKind["Exit"] = "Exit";
    PlannerEventKind["ApiSchema"] = "ApiSchema";
    PlannerEventKind["Introspect"] = "Introspect";
    PlannerEventKind["Signature"] = "Signature";
    PlannerEventKind["Subgraphs"] = "Subgraphs";
    PlannerEventKind["GetHeapStatistics"] = "GetHeapStatistics";
})(PlannerEventKind || (PlannerEventKind = {}));
const isGraphQLErrorExt = (error) => error.name === "GraphQLError" || error.name === "CheckFailed";
const intoSerializableError = (error) => {
    const { name, message, stack, validationError = false, } = error;
    return {
        name,
        message,
        stack,
        validationError,
    };
};
const intoCauseError = (error) => {
    const { locations, message, extensions } = error;
    return {
        locations,
        message,
        extensions,
    };
};
const intoSerializableGraphQLErrorExt = (error) => {
    const { message, locations, path, extensions } = error.toJSON();
    const { nodes, source, positions, originalError, name, validationError = false, } = error;
    const causes = error.causes;
    return {
        name,
        message,
        locations,
        path,
        extensions,
        nodes,
        source,
        positions,
        originalError: originalError === undefined
            ? originalError
            : intoSerializableError(originalError),
        causes: causes === undefined ? causes : causes.map(intoCauseError),
        validationError,
    };
};
const send = async (payload) => {
    logger.trace(`plan_worker: sending payload ${JSON.stringify(payload)}`);
    await Deno.core.ops.send(payload);
};
const receive = async () => await Deno.core.ops.receive();
let planners = new Map();
const updateQueryPlanner = (schema, options, schemaId) => {
    try {
        planners.set(schemaId, new bridge.BridgeQueryPlanner(schema, options));
        return {
            data: {
                queryPlan: { kind: "QueryPlan", node: null },
                formattedQueryPlan: "QueryPlan {}",
            },
            usageReporting: {
                statsReportKey: "",
                referencedFieldsByType: {},
            },
        };
    }
    catch (err) {
        const errorArray = Array.isArray(err) ? err : [err];
        const errors = errorArray.map((err) => {
            if (isGraphQLErrorExt(err)) {
                return intoSerializableGraphQLErrorExt(err);
            }
            else {
                return intoSerializableError(err);
            }
        });
        return { errors };
    }
};
async function run() {
    while (true) {
        let messageId = "";
        try {
            const { id, payload: event } = await receive();
            messageId = id;
            try {
                switch (event === null || event === void 0 ? void 0 : event.kind) {
                    case PlannerEventKind.UpdateSchema:
                        const updateResult = updateQueryPlanner(event.schema, event.config, event.schemaId);
                        await send({ id, payload: updateResult });
                        break;
                    case PlannerEventKind.Plan:
                        const planResult = planners
                            .get(event.schemaId)
                            .plan(event.query, event.operationName, event.options);
                        await send({ id, payload: planResult });
                        break;
                    case PlannerEventKind.ApiSchema:
                        const apiSchemaResult = planners.get(event.schemaId).getApiSchema();
                        const payload = { schema: apiSchemaResult };
                        await send({ id, payload });
                        break;
                    case PlannerEventKind.Introspect:
                        const introspectResult = planners
                            .get(event.schemaId)
                            .introspect(event.query);
                        await send({ id, payload: introspectResult });
                        break;
                    case PlannerEventKind.Signature:
                        const signature = planners
                            .get(event.schemaId)
                            .operationSignature(event.query, event.operationName);
                        await send({ id, payload: signature });
                        break;
                    case PlannerEventKind.Subgraphs:
                        const subgraphs = planners.get(event.schemaId).subgraphs();
                        await send({ id, payload: subgraphs });
                        break;
                    case PlannerEventKind.GetHeapStatistics:
                        const mem = memoryUsage();
                        const result = {
                            heapTotal: mem.heapTotal,
                            heapUsed: mem.heapUsed,
                            external: mem.external,
                        };
                        await send({ id, payload: result });
                        break;
                    case PlannerEventKind.Exit:
                        planners.delete(event.schemaId);
                        if (planners.size == 0) {
                            return;
                        }
                        else {
                            break;
                        }
                    default:
                        logger.warn(`unknown message received: ${JSON.stringify(event)}\n`);
                        break;
                }
            }
            catch (e) {
                logger.warn(`an error happened in the worker runtime ${e}\n`);
                const unexpectedError = {
                    name: e.name || "unknown",
                    message: e.message || "",
                    extensions: {
                        code: "QUERY_PLANNING_FAILED",
                        exception: {
                            stacktrace: e.toString(),
                        },
                    },
                };
                await send({
                    id,
                    payload: {
                        errors: [unexpectedError],
                        usageReporting: {
                            statsReportKey: "",
                            referencedFieldsByType: {},
                        },
                    },
                });
            }
        }
        catch (e) {
            logger.warn(`plan_worker: an unknown error occurred ${e}\n`);
            const unexpectedError = {
                name: e.name || "unknown",
                message: e.message || "",
                extensions: {
                    code: "QUERY_PLANNING_FAILED",
                    exception: {
                        stacktrace: e.toString(),
                    },
                },
            };
            await send({
                id: messageId,
                payload: {
                    errors: [unexpectedError],
                    usageReporting: {
                        statsReportKey: "",
                        referencedFieldsByType: {},
                    },
                },
            });
        }
    }
}
run();
//# sourceMappingURL=plan_worker.js.map