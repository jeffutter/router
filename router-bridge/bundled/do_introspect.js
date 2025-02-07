"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
let opResult;
if (!sdl) {
    opResult = {
        Err: [{ message: "Error in JS-Rust-land: SDL is empty." }],
    };
}
else {
    try {
        opResult = { Ok: bridge.batchIntrospect(sdl, queries, config) };
    }
    catch (err) {
        opResult = { Err: err };
    }
}
opResult;
//# sourceMappingURL=do_introspect.js.map