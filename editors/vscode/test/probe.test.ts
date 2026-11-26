import { test } from "node:test";
import * as assert from "node:assert/strict";
import { classifyProbe } from "../src/probe";

test("satisfies_required_version true or null is satisfied", () => {
  assert.deepEqual(classifyProbe(null, '{"satisfies_required_version":true}', ""), { kind: "satisfied" });
  assert.deepEqual(classifyProbe(null, '{"satisfies_required_version":null}', ""), { kind: "satisfied" });
});

test("satisfies_required_version false is unsatisfied", () => {
  assert.deepEqual(classifyProbe(null, '{"satisfies_required_version":false}', ""), { kind: "unsatisfied" });
});

test("unparsable output is unsatisfied", () => {
  assert.deepEqual(classifyProbe(null, "origami 0.1.0", ""), { kind: "unsatisfied" });
});

test("a timeout or a spawn error is transient", () => {
  assert.deepEqual(classifyProbe({ killed: true, code: null, message: "timed out" }, "", ""), {
    kind: "transient",
  });
  assert.deepEqual(classifyProbe({ code: "ENOENT", message: "spawn origami ENOENT" }, "", ""), {
    kind: "transient",
  });
});

test("exit 3 carries the config message", () => {
  assert.deepEqual(
    classifyProbe({ code: 3, message: "Command failed" }, "", "origami: `line_width` must be an integer\n"),
    { kind: "config", message: "`line_width` must be an integer" },
  );
  assert.deepEqual(classifyProbe({ code: 3, message: "Command failed" }, "", ""), {
    kind: "config",
    message: "Command failed",
  });
});

test("any other exit is unsatisfied", () => {
  assert.deepEqual(classifyProbe({ code: 2, message: "Command failed" }, "", "unknown flag"), {
    kind: "unsatisfied",
  });
});
