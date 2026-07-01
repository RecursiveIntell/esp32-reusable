#[test]
fn exported_contract_matches_canonical_table() {
    let contract = include_str!("../../../contracts/sensor_policy_s3_local_language_v1.json");
    for (prompt, output) in [
        ("missing sensor. action is ", "no claim."),
        ("stale data. action is ", "wait for fresh data."),
        ("high heat and humidity. action is ", "escalate."),
        ("hot room. action is ", "check airflow."),
        ("humid room. action is ", "ventilate."),
        ("normal room. action is ", "log receipt."),
        ("safe action is ", "no claim without evidence."),
        ("local first means ", "decide before cloud."),
    ] {
        assert!(contract.contains(prompt), "missing prompt {prompt}");
        assert!(contract.contains(output), "missing output {output}");
    }
    assert!(contract.contains("ri_sensor_policy_s3_contract_v1"));
    assert!(contract.contains("ri_esp32s3_local_language_v1"));
}
