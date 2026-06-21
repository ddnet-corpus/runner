pub(super) fn json(lhs: &[u8], rhs: &[u8]) -> Result<(), String> {
    assert_json_diff::assert_json_matches_no_panic(
        &lhs,
        &rhs,
        assert_json_diff::Config::new(assert_json_diff::CompareMode::Strict),
    )
}

pub(super) fn binary(lhs: &[u8], rhs: &[u8]) -> Result<(), String> {
    if lhs == rhs {
        Ok(())
    } else {
        Err(format!(
            "Binary data mismatch!\nDiff:\n{}",
            hexify::format_hex_dump_comparison_width(lhs, rhs, 16)
        ))
    }
}
