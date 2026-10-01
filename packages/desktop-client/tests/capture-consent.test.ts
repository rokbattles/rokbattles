import assert from "node:assert/strict";
import { test } from "node:test";

import { canChangeCaptureConsent } from "../src/lib/capture-consent.ts";

test("an unsupported package can revoke existing capture consent", () => {
  assert.equal(canChangeCaptureConsent({ capture_supported: false, capture_opt_in: true }), true);
});

test("an unsupported package cannot enable capture", () => {
  assert.equal(canChangeCaptureConsent({ capture_supported: false, capture_opt_in: false }), false);
});

test("supported packages can enable and revoke capture consent", () => {
  for (const capture_opt_in of [false, true]) {
    assert.equal(canChangeCaptureConsent({ capture_supported: true, capture_opt_in }), true);
  }
});
