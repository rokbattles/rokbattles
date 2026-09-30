import assert from "node:assert/strict";
import { beforeEach, mock, test } from "node:test";

const actions = [];
const banners = [];
const open = mock.fn();
const invoke = mock.fn();

// Exercise the real hook's handlers without a browser or a native picker. React
// rendering and native dialog timing remain separate integration checks.
mock.module("react", {
  exports: {
    useCallback: (callback) => callback,
    useEffect: () => {},
    useReducer: (_reducer, initialState) => [initialState, (action) => actions.push(action)],
    useRef: (value) => ({ current: value }),
  },
});
mock.module("@tauri-apps/plugin-dialog", { exports: { open } });
mock.module("@tauri-apps/api/core", { exports: { invoke } });

const { useWatchedDirectories } = await import("../src/hooks/useWatchedDirectories.ts");

function deferred() {
  let resolve;
  let reject;
  const promise = new Promise((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
}

function createHook() {
  // biome-ignore lint/correctness/useHookAtTopLevel: React hooks are stubbed for handler unit tests.
  return useWatchedDirectories((...banner) => banners.push(banner));
}

beforeEach(() => {
  actions.length = 0;
  banners.length = 0;
  open.mock.resetCalls();
  invoke.mock.resetCalls();
  open.mock.mockImplementation(async () => null);
  invoke.mock.mockImplementation(async (command) => {
    // A paused/busy watcher must never enter the manual picker lifecycle.
    if (command === "pause_watcher" || command === "resume_watcher") {
      return new Promise(() => {});
    }
    return [];
  });
});

test("opens immediately without waiting for a busy watcher, and cancel preserves its state", async () => {
  const selection = deferred();
  open.mock.mockImplementation(() => selection.promise);
  const hook = createHook();
  const adding = hook.handleAdd();

  assert.equal(open.mock.callCount(), 1);
  assert.deepEqual(open.mock.calls[0].arguments, [{ multiple: true, directory: true }]);
  assert.equal(invoke.mock.callCount(), 0);
  assert.deepEqual(actions, [{ type: "addStarted" }]);

  selection.resolve(null);
  await adding;
  assert.equal(invoke.mock.callCount(), 0);
  assert.deepEqual(actions.at(-1), { type: "addFinished" });
});

test("ignores repeated clicks before a render and permits reopening after cancel", async () => {
  const selection = deferred();
  open.mock.mockImplementation(() => selection.promise);
  const hook = createHook();
  const adding = hook.handleAdd();
  await hook.handleAdd();
  assert.equal(open.mock.callCount(), 1);

  selection.resolve(null);
  await adding;
  await hook.handleAdd();
  assert.equal(open.mock.callCount(), 2);
});

test("keeps repeated clicks guarded while selected directories are being saved", async () => {
  const saved = deferred();
  open.mock.mockImplementation(async () => ["/mail/one", "/mail/two"]);
  invoke.mock.mockImplementation((command) =>
    command === "add_dir" ? saved.promise : Promise.resolve(["/mail/one", "/mail/two"])
  );
  const hook = createHook();
  const adding = hook.handleAdd();
  await Promise.resolve();
  await hook.handleAdd();
  assert.equal(open.mock.callCount(), 1);
  assert.deepEqual(invoke.mock.calls[0].arguments, [
    "add_dir",
    { paths: ["/mail/one", "/mail/two"] },
  ]);

  saved.resolve([]);
  await adding;
  assert.deepEqual(
    invoke.mock.calls.map((call) => call.arguments[0]),
    ["add_dir", "list_dirs"]
  );
  assert.deepEqual(actions.at(-2), { type: "dirsLoaded", dirs: ["/mail/one", "/mail/two"] });
  assert.deepEqual(actions.at(-1), { type: "addFinished" });
});

test("supports a single selected directory without changing watcher state", async () => {
  open.mock.mockImplementation(async () => "/mail/single");
  await createHook().handleAdd();
  assert.deepEqual(invoke.mock.calls[0].arguments, ["add_dir", { paths: ["/mail/single"] }]);
  assert.deepEqual(
    invoke.mock.calls.map((call) => call.arguments[0]),
    ["add_dir", "list_dirs"]
  );
});

test("an empty selection is a no-op", async () => {
  open.mock.mockImplementation(async () => []);
  await createHook().handleAdd();
  assert.equal(invoke.mock.callCount(), 0);
  assert.deepEqual(actions.at(-1), { type: "addFinished" });
});

for (const failingStep of ["picker", "save"]) {
  test(`${failingStep} failures surface an error, release the guard, and allow retry`, async (t) => {
    t.mock.method(console, "error", () => {});
    const failure = new Error("unavailable");
    open.mock.mockImplementation(async () => {
      if (failingStep === "picker") throw failure;
      return "/mail/retry";
    });
    invoke.mock.mockImplementation(async () => {
      throw failure;
    });
    const hook = createHook();
    await hook.handleAdd();
    assert.equal(banners.length, 1);
    assert.equal(banners[0][0], "error");
    assert.deepEqual(actions.at(-1), { type: "addFinished" });
    assert.ok(invoke.mock.calls.every((call) => call.arguments[0] === "add_dir"));

    open.mock.mockImplementation(async () => null);
    await hook.handleAdd();
    assert.equal(open.mock.callCount(), 2);
    assert.equal(banners.length, 1);
  });
}

for (const oldLoadFails of [false, true]) {
  test(`a late ${oldLoadFails ? "failed" : "successful"} list cannot overwrite newer directories`, async (t) => {
    t.mock.method(console, "error", () => {});
    const oldList = deferred();
    let lists = 0;
    invoke.mock.mockImplementation(async (command) => {
      if (command !== "list_dirs") return [];
      lists += 1;
      return lists === 1 ? oldList.promise : ["/mail/new"];
    });
    const hook = createHook();
    const older = hook.handleRemove("/mail/old");
    await Promise.resolve();
    await hook.handleRemove("/mail/other");
    const actionsBeforeOldResponse = actions.slice();
    if (oldLoadFails) oldList.reject(new Error("unavailable drive"));
    else oldList.resolve(["/mail/stale"]);
    await older;
    assert.deepEqual(actions, actionsBeforeOldResponse);
    assert.deepEqual(actions.at(-1), { type: "dirsLoaded", dirs: ["/mail/new"] });
  });
}
