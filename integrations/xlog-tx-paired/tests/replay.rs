// SPDX-License-Identifier: GPL-3.0-only
#[test]
fn ft8_cq() {
    xnexus_tx_paired::replay("ft8-cq");
}
#[test]
fn ft8_grid() {
    xnexus_tx_paired::replay("ft8-grid");
}
#[test]
fn ft8_report() {
    xnexus_tx_paired::replay("ft8-report");
}
#[test]
fn ft8_rreport() {
    xnexus_tx_paired::replay("ft8-rreport");
}
#[test]
fn ft8_rrr() {
    xnexus_tx_paired::replay("ft8-rrr");
}
#[test]
fn ft8_rr73() {
    xnexus_tx_paired::replay("ft8-rr73");
}
#[test]
fn ft8_73() {
    xnexus_tx_paired::replay("ft8-73");
}
#[test]
fn ft4_cq() {
    xnexus_tx_paired::replay("ft4-cq");
}
#[test]
fn ft4_grid() {
    xnexus_tx_paired::replay("ft4-grid");
}
#[test]
fn ft4_report() {
    xnexus_tx_paired::replay("ft4-report");
}
#[test]
fn ft4_rreport() {
    xnexus_tx_paired::replay("ft4-rreport");
}
#[test]
fn ft4_rrr() {
    xnexus_tx_paired::replay("ft4-rrr");
}
#[test]
fn ft4_rr73() {
    xnexus_tx_paired::replay("ft4-rr73");
}
#[test]
fn ft4_73() {
    xnexus_tx_paired::replay("ft4-73");
}
#[test]
fn corrupt_pcm_rejected() {
    xnexus_tx_paired::corrupt_pcm_rejected();
}
#[test]
fn corrupt_manifest_rejected() {
    xnexus_tx_paired::corrupt_manifest_rejected();
}
#[test]
fn whitespace_manifest_changes_body_and_substitution_is_refused() {
    xnexus_tx_paired::whitespace_manifest_changes_body_and_substitution_is_refused();
}
#[test]
fn changed_context_at_fake_effect_refuses_without_advancing() {
    xnexus_tx_paired::changed_context_at_fake_effect_refuses_without_advancing();
}
