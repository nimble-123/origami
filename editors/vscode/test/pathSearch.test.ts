import { test } from "node:test";
import * as assert from "node:assert/strict";
import { findOnPath } from "../src/pathSearch";

const files = (...present: string[]) => (p: string) => present.includes(p);

test("posix: first directory on PATH wins, result is absolute", () => {
  const found = findOnPath("origami", {
    env: { PATH: "/usr/local/bin:/opt/bin:/usr/bin" },
    platform: "linux",
    isFile: files("/opt/bin/origami", "/usr/bin/origami"),
  });
  assert.equal(found, "/opt/bin/origami");
});

test("posix: empty and relative PATH entries are skipped", () => {
  const found = findOnPath("origami", {
    env: { PATH: ":.:bin:/usr/bin" },
    platform: "linux",
    isFile: files("origami", "./origami", "bin/origami", "/usr/bin/origami"),
  });
  assert.equal(found, "/usr/bin/origami");
});

test("not on PATH", () => {
  assert.equal(
    findOnPath("origami", { env: { PATH: "/usr/bin" }, platform: "linux", isFile: files() }),
    undefined,
  );
  assert.equal(findOnPath("origami", { env: {}, platform: "linux", isFile: files() }), undefined);
});

test("windows: PATHEXT is applied, only .exe and .com are spawnable", () => {
  const found = findOnPath("origami", {
    env: { PATH: "C:\\tools;C:\\bin", PATHEXT: ".COM;.EXE;.BAT;.CMD" },
    platform: "win32",
    isFile: files("C:\\tools\\origami.cmd", "C:\\bin\\origami.EXE"),
  });
  assert.equal(found, "C:\\bin\\origami.EXE");
});

test("windows: quoted entries are unquoted, relative ones skipped", () => {
  const found = findOnPath("origami", {
    env: { Path: '.;"C:\\Program Files\\origami"' },
    platform: "win32",
    isFile: files("origami.EXE", ".\\origami.EXE", "C:\\Program Files\\origami\\origami.EXE"),
  });
  assert.equal(found, "C:\\Program Files\\origami\\origami.EXE");
});
