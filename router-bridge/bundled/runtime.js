logger = {
    trace: (message) => Deno.core.ops.log_trace(`${message.toString()}\n`),
    debug: (message) => Deno.core.ops.log_debug(`${message.toString()}\n`),
    info: (message) => Deno.core.ops.log_info(`${message.toString()}\n`),
    warn: (message) => Deno.core.ops.log_warn(`${message.toString()}\n`),
    error: (message) => Deno.core.ops.log_error(`${message.toString()}\n`),
};
function print(value) {
    Deno.core.print(`${value.toString()}\n`);
}
crypto = {
    getRandomValues: (arg) => {
        Deno.core.ops.op_crypto_get_random_values(arg);
        return arg;
    },
};
node_fetch_1 = {};
process = { argv: [], env: { NODE_ENV: "production" } };
global = {};
exports = {};
//# sourceMappingURL=runtime.js.map