import assert from "node:assert/strict";
import test from "node:test";
import { checkoutLink } from "../lib/checkout-link.ts";

test("preserves HTTPS provider links and checkout query parameters", () => {
  const url = "https://checkout.stripe.com/c/pay/cs_test_example?client_reference_id=moladt";
  assert.equal(checkoutLink(url), url);
  assert.equal(checkoutLink("https://buy.stripe.com/example"), "https://buy.stripe.com/example");
});

test("rejects executable, insecure, relative, credential-bearing and invalid destinations", () => {
  for (const value of [
    "javascript:alert(1)", "data:text/html,test", "http://checkout.stripe.com/",
    "//checkout.stripe.com/pay", "/checkout", "https://user:secret@example.com/pay",
    "not a URL", "", null, {}, 42,
  ]) {
    assert.equal(checkoutLink(value), null);
  }
});
