from conftest import poll_for_descendants

# Nav rail labels (short) -> the breadcrumb title each one navigates to (see i18n.rs's
# NavX/ScreenTitleX pairs — they're not always the same string, e.g. "Fila" vs. "Fila de
# exportação"). App launches in pt-BR (OcaApp::new), so these are the pt-BR strings.
NAV_LABEL_TO_SCREEN_TITLE = {
    "Editor": "Editor",
    "Mídia": "Mídia",
    "Fila": "Fila de exportação",
    "Ajustes": "Ajustes",
    "Início": "Início",
}


def test_navigating_to_each_screen_updates_the_breadcrumb(oca_window):
    for nav_label, screen_title in NAV_LABEL_TO_SCREEN_TITLE.items():
        oca_window.child_window(title=nav_label, control_type="Button").click_input()

        assert oca_window.exists()  # still alive: no panic mid-navigation
        # descendants(), not child_window().exists() — some screens repeat their breadcrumb
        # title as their own on-screen heading too (e.g. the Fila screen's "Fila de
        # exportação"), so child_window() would raise ElementAmbiguousError on a screen where
        # navigation actually worked correctly. Polled rather than a single snapshot — the
        # app only repaints every 200ms while idle (OcaApp::ui's request_repaint_after), so
        # the click's effect isn't necessarily visible to UI Automation yet the instant
        # click_input() returns.
        matches = poll_for_descendants(oca_window, screen_title, timeout_secs=3.0)
        assert matches, f"no Text control titled {screen_title!r} after clicking {nav_label!r}"
