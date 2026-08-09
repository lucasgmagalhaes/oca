use eframe::egui::{self, RichText};

use crate::app::{NivelaApp, LUFS_PROFILES};
use crate::screens::widgets;
use crate::theme;

pub fn show(app: &mut NivelaApp, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(20.0);
        ui.label(RichText::new("Preferências").size(20.0).strong());
        ui.add_space(16.0);
        ui.set_max_width(640.0);

        widgets::card_frame().show(ui, |ui| {
            ui.label(RichText::new("Áudio").size(11.0).color(theme::TEXT_MUTED).strong());
            ui.add_space(6.0);
            ui.label("Perfil de normalização padrão");
            ui.horizontal(|ui| {
                for (i, (label, _)) in LUFS_PROFILES.iter().enumerate() {
                    if ui.selectable_label(app.prefs.lufs_profile == i, *label).clicked() {
                        app.prefs.lufs_profile = i;
                    }
                }
            });
            ui.add_space(6.0);
            ui.checkbox(&mut app.prefs.true_peak_limiter, "Limitador de true peak ativado (-1.0 dBTP)");
        });
        ui.add_space(14.0);

        widgets::card_frame().show(ui, |ui| {
            ui.label(RichText::new("Exportação").size(11.0).color(theme::TEXT_MUTED).strong());
            ui.add_space(6.0);
            ui.label("Workers de exportação em segundo plano");
            ui.horizontal(|ui| {
                for w in [1u8, 2, 4, 8] {
                    if ui.selectable_label(app.prefs.export_workers == w, format!("{w}")).clicked() {
                        app.prefs.export_workers = w;
                    }
                }
            });
            ui.add_space(10.0);
            ui.label("Pasta de saída padrão");
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut app.prefs.output_folder).desired_width(400.0));
                let _ = ui.button("Procurar");
            });
        });
        ui.add_space(14.0);

        widgets::card_frame().show(ui, |ui| {
            ui.label(RichText::new("Projeto").size(11.0).color(theme::TEXT_MUTED).strong());
            ui.add_space(6.0);
            ui.label("Intervalo de autosave");
            ui.horizontal(|ui| {
                for m in [1u8, 5, 10] {
                    if ui.selectable_label(app.prefs.autosave_minutes == m, format!("{m} min")).clicked() {
                        app.prefs.autosave_minutes = m;
                    }
                }
            });
        });
        ui.add_space(14.0);

        widgets::card_frame().show(ui, |ui| {
            ui.label(RichText::new("Atalhos de teclado").size(11.0).color(theme::TEXT_MUTED).strong());
            ui.add_space(6.0);
            egui::Grid::new("shortcuts_table")
                .num_columns(2)
                .spacing(egui::vec2(24.0, 6.0))
                .striped(false)
                .show(ui, |ui| {
                    for (action, key) in [
                        ("Dividir clipe (split)", "S"),
                        ("Cortar", "X"),
                        ("Play / Pause", "Espaço"),
                        ("Marcar entrada / saída", "I / O"),
                        ("Enviar para fila de exportação", "Ctrl+E"),
                    ] {
                        ui.label(action);
                        ui.label(RichText::new(key).color(theme::TEXT_MUTED).monospace());
                        ui.end_row();
                    }
                });
        });
    });
}
