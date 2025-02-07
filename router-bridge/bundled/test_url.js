const assertEq = (a, b) => {
    if (a !== b) {
        throw `${a} is not equal to ${b}`;
    }
};
const url = new URL("https://www.test.com/test2");
assertEq("/test2", url.pathname);
assertEq("www.test.com", url.hostname);
assertEq("https", url.scheme);
//# sourceMappingURL=test_url.js.map