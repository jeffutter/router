const rnds8 = new Uint8Array(16);
const randomValue = crypto.getRandomValues(rnds8);
if (!randomValue) {
    throw "couldn't use crypto.getRandomValues";
}
//# sourceMappingURL=test_get_random_values.js.map