let logFunction;
var CommandKind;
(function (CommandKind) {
    CommandKind["Trace"] = "Trace";
    CommandKind["Debug"] = "Debug";
    CommandKind["Info"] = "Info";
    CommandKind["Warn"] = "Warn";
    CommandKind["Error"] = "Error";
    CommandKind["Exit"] = "Exit";
})(CommandKind || (CommandKind = {}));
const send = async (result) => {
    await Deno.core.opAsync("send", result);
};
const receive = async () => await Deno.core.opAsync("receive");
async function run() {
    while (true) {
        try {
            const event = await receive();
            const { id, payload: { kind, message }, } = event;
            switch (kind) {
                case CommandKind.Trace:
                    logger.trace(message);
                    await send({ id, payload: true });
                    break;
                case CommandKind.Debug:
                    logger.debug(message);
                    await send({ id, payload: true });
                    break;
                case CommandKind.Info:
                    logger.info(message);
                    await send({ id, payload: true });
                    break;
                case CommandKind.Warn:
                    logger.warn(message);
                    await send({ id, payload: true });
                    break;
                case CommandKind.Error:
                    logger.error(message);
                    await send({ id, payload: true });
                    break;
                case CommandKind.Exit:
                    await send({ id, payload: true });
                    return;
                default:
                    logger.error(`unknown message received: ${JSON.stringify(event)}\n`);
                    break;
            }
        }
        catch (e) {
            logger.error(`an unknown error occured ${e}\n`);
        }
    }
}
run();
//# sourceMappingURL=test_logger_worker.js.map