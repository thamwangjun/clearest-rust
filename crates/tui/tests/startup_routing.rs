//! Startup routing tests — verifies that show() opens the Welcome page,
//! not ProviderSetup. Covers D-07 from phase 2 CONTEXT.md.

use claurst_tui::onboarding_dialog::{OnboardingDialogState, OnboardingPage};

#[test]
fn show_starts_at_welcome_page() {
    let mut dialog = OnboardingDialogState::new();
    dialog.show();
    assert!(dialog.visible, "dialog should be visible after show()");
    assert_eq!(
        dialog.page,
        OnboardingPage::Welcome,
        "show() must start at Welcome, not ProviderSetup"
    );
}
