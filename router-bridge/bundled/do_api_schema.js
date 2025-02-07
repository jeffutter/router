"use strict";
var _a;
Object.defineProperty(exports, "__esModule", { value: true });
const result = bridge.apiSchema(sdl, { graphqlValidation });
let opResult;
if (((_a = result.errors) === null || _a === void 0 ? void 0 : _a.length) > 0) {
    opResult = { Err: result.errors };
}
else {
    opResult = { Ok: result.data };
}
opResult;
//# sourceMappingURL=do_api_schema.js.map