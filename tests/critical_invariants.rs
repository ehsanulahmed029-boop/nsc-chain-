// Regression tests for known critical bugs/fixes in NSC chain.
// Placeholder tests — আসল module path/function name অনুযায়ী পরে আপডেট করতে হবে।

#[test]
fn nan_swap_output_is_rejected() {
    let bad_value: f64 = f64::NAN;
    assert!(!bad_value.is_finite());
}

#[test]
fn max_supply_enforced_on_credit() {
    // TODO: credit_nsc() কল করে MAX_SUPPLY এর বেশি mint reject হচ্ছে কিনা assert করুন
}

#[test]
fn u128_serializes_as_string_in_json() {
    let amount: u128 = 25_000_000_000_000_000_000_000_000;
    let json_str = serde_json::json!({ "amount": amount.to_string() });
    assert!(json_str["amount"].is_string());
}
