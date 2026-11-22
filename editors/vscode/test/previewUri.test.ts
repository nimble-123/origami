import { test } from "node:test";
import * as assert from "node:assert/strict";
import { decode, encode } from "../src/previewUri";

test("round trip", () => {
  for (const side of ["before", "after"] as const) {
    const ref = { id: 7, side, name: "zcl_a.clas.abap" };
    assert.deepEqual(decode(encode(ref)), ref);
  }
});

test("unsafe characters in the name become underscores", () => {
  const uri = encode({ id: 1, side: "after", name: "#dmo#cl x?.clas.abap" });
  assert.equal(uri, "origami-preview:/_dmo_cl_x_.clas.abap?side=after&id=1");
  assert.equal(decode(uri)?.name, "_dmo_cl_x_.clas.abap");
});

test("accepts a percent-encoded query", () => {
  assert.deepEqual(decode("origami-preview:/a.abap?side%3Dbefore%26id%3D3"), {
    id: 3,
    side: "before",
    name: "a.abap",
  });
});

test("rejects other URIs", () => {
  assert.equal(decode("file:///a.abap"), undefined);
  assert.equal(decode("origami-preview:/a.abap"), undefined);
  assert.equal(decode("origami-preview:/a.abap?side=left&id=1"), undefined);
  assert.equal(decode("origami-preview:/a.abap?side=after&id=x"), undefined);
});
