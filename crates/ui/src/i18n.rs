//! All translatable UI text lives here, not in the screen modules — `core` stays
//! locale-neutral (see e.g. [`avcore::project::Recency`]) and this module is the only
//! place that turns its raw data into display strings.
//!
//! [`Text`] is the static-string catalog (nav labels, section headers, button text — see the
//! `text_catalog!` invocation below for the full list); the functions below it
//! ([`recency_label`], [`track_summary`], [`job_status_label`], [`job_detail_line`]) handle
//! the handful of strings that need pluralization or interpolation instead of a flat lookup.

use avcore::export::{ExportJob, ExportJobStatus};
use avcore::project::Recency;

use crate::app::Screen;

/// A language the UI can be displayed in. Stored on [`crate::app::OcaApp`] and switched
/// at runtime from the Ajustes screen — nothing here requires a restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    PtBr,
    En,
}

impl Locale {
    /// Every supported locale, for populating the language switcher.
    pub const ALL: [Locale; 2] = [Locale::PtBr, Locale::En];

    /// The language's own name, as it should appear in its own language switcher entry.
    pub fn native_name(self) -> &'static str {
        match self {
            Locale::PtBr => "Português",
            Locale::En => "English",
        }
    }
}

/// Defines the [`Text`] enum from a `Variant: pt_br = "...", en = "...";` list — one variant
/// per translatable UI string, with [`Text::tr`] doing the lookup. Keeps the two locales'
/// strings pinned next to each other so adding a string can't accidentally ship with only
/// one language filled in.
macro_rules! text_catalog {
    ($( $variant:ident : pt_br = $pt:expr, en = $en:expr ; )*) => {
        /// A translatable UI string. One variant per string used anywhere in the app — call
        /// [`Text::tr`] with the current [`Locale`] to get the display text.
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub enum Text {
            $( $variant, )*
        }

        impl Text {
            /// Every variant, for exhaustive checks like "no string is empty in any locale".
            #[cfg(test)]
            pub const ALL: &'static [Text] = &[ $( Text::$variant, )* ];

            /// Looks up this string's text in `locale`.
            pub fn tr(self, locale: Locale) -> &'static str {
                match locale {
                    Locale::PtBr => match self { $( Text::$variant => $pt, )* },
                    Locale::En => match self { $( Text::$variant => $en, )* },
                }
            }
        }
    };
}

text_catalog! {
    AppName: pt_br = "oca", en = "oca";

    NavHome: pt_br = "Início", en = "Home";
    NavEditor: pt_br = "Editor", en = "Editor";
    NavLibrary: pt_br = "Mídia", en = "Media";
    NavQueue: pt_br = "Fila", en = "Queue";
    NavPrefs: pt_br = "Ajustes", en = "Settings";

    ScreenTitleHome: pt_br = "Início", en = "Home";
    ScreenTitleEditor: pt_br = "Editor", en = "Editor";
    ScreenTitleLibrary: pt_br = "Mídia", en = "Media";
    ScreenTitleQueue: pt_br = "Fila de exportação", en = "Export queue";
    ScreenTitlePrefs: pt_br = "Ajustes", en = "Settings";
    UnsavedChanges: pt_br = "Alterações não salvas", en = "Unsaved changes";

    HomeTitle: pt_br = "Projetos recentes", en = "Recent projects";
    HomeSubtitle: pt_br = "Continue de onde parou ou comece um projeto novo.", en = "Continue where you left off or start a new project.";
    NewProject: pt_br = "＋ Novo projeto", en = "＋ New project";
    OpenProject: pt_br = "Abrir projeto…", en = "Open project…";
    UntitledProject: pt_br = "Projeto sem título", en = "Untitled project";
    Save: pt_br = "💾 Salvar", en = "💾 Save";
    ClipsUnit: pt_br = "clipes", en = "clips";
    ClipUnitSingular: pt_br = "clipe", en = "clip";

    ToolSelect: pt_br = "Selecionar", en = "Select";
    ToolCut: pt_br = "Cortar / Split", en = "Cut / Split";
    ToolTrim: pt_br = "Aparar", en = "Trim";
    Export: pt_br = "⭳ Exportar", en = "⭳ Export";
    MediaLibrary: pt_br = "Biblioteca de mídia", en = "Media library";
    SelectedClip: pt_br = "Clipe selecionado", en = "Selected clip";
    NoClipSelected: pt_br = "Nenhum clipe selecionado", en = "No clip selected";
    PropCodec: pt_br = "Codec", en = "Codec";
    PropSourceBitrate: pt_br = "Bitrate fonte", en = "Source bitrate";
    PropResolution: pt_br = "Resolução", en = "Resolution";
    PropFps: pt_br = "FPS", en = "FPS";
    PropLoudness: pt_br = "Loudness", en = "Loudness";
    PropProxy: pt_br = "Proxy de edição", en = "Editing proxy";
    ProxyPresent: pt_br = "540p pronto", en = "540p ready";
    ProxyAbsent: pt_br = "usando original", en = "using original";
    OnExport: pt_br = "Ao exportar", en = "On export";
    NormalizeTo: pt_br = "Normalizar", en = "Normalize";
    BitrateFromSource: pt_br = "Bitrate = fonte", en = "Bitrate = source";
    ExportAutoNote: pt_br = "loudnorm 2-pass + true peak limiter, aplicado automático no export — sem ajuste manual por clipe.", en = "loudnorm 2-pass + true peak limiter, applied automatically on export — no per-clip manual adjustment.";
    Timeline: pt_br = "🔍 Timeline", en = "🔍 Timeline";
    TimelineEmpty: pt_br = "Este projeto ainda não tem clipes na timeline.", en = "This project doesn't have any clips on the timeline yet.";

    LibraryTitle: pt_br = "Biblioteca de mídia", en = "Media library";
    ImportFiles: pt_br = "⭱ Importar arquivos", en = "⭱ Import files";
    LibraryEmpty: pt_br = "Nenhum arquivo importado neste projeto ainda.", en = "No files imported into this project yet.";
    ProxyReady: pt_br = "Proxy 540p", en = "540p proxy";

    QueueTitle: pt_br = "Fila de exportação", en = "Export queue";
    AddExport: pt_br = "＋ Adicionar exportação", en = "＋ Add export";
    AddExportNeedsClip: pt_br = "Selecione um clipe na Mídia ou no Editor primeiro", en = "Select a clip in Mídia or the Editor first";
    ConcurrentWorkers: pt_br = "Workers simultâneos", en = "Concurrent workers";
    QueueSubtitle: pt_br = "A edição continua responsiva enquanto os jobs renderizam em segundo plano. A fila persiste entre sessões.", en = "Editing stays responsive while jobs render in the background. The queue persists across sessions.";
    QueueTechNote: pt_br = "Nota técnica: cada job é um snapshot (bitrate/perfil/destino) tirado no momento em que entra na fila — mudanças no projeto ativo depois disso não afetam o job. Render roda em worker separado da UI (tokio::mpsc); 1 worker por padrão, configurável em Preferências.", en = "Technical note: each job is a snapshot (bitrate/profile/destination) taken the moment it enters the queue — later changes to the active project don't affect the job. Rendering runs in a worker separate from the UI (tokio::mpsc); 1 worker by default, configurable in Preferences.";
    StatusRendering: pt_br = "Renderizando", en = "Rendering";
    StatusQueued: pt_br = "Na fila", en = "Queued";
    StatusPaused: pt_br = "Pausado", en = "Paused";
    StatusDone: pt_br = "Concluído", en = "Done";
    StatusFailed: pt_br = "Falhou", en = "Failed";
    CancelJob: pt_br = "Cancelar", en = "Cancel";
    RemoveJob: pt_br = "Remover", en = "Remove";
    Resume: pt_br = "Retomar", en = "Resume";
    OpenFolder: pt_br = "Abrir pasta", en = "Open folder";
    RetryExport: pt_br = "↻ Tentar novamente", en = "↻ Retry";
    ErrorPrefix: pt_br = "Erro", en = "Error";

    PrefsTitle: pt_br = "Preferências", en = "Preferences";
    PrefsLanguage: pt_br = "Idioma", en = "Language";
    PrefsAudio: pt_br = "Áudio", en = "Audio";
    PrefsNormalizationProfile: pt_br = "Perfil de normalização padrão", en = "Default normalization profile";
    PrefsTruePeakLimiter: pt_br = "Limitador de true peak ativado (-1.0 dBTP)", en = "True peak limiter enabled (-1.0 dBTP)";
    PrefsExport: pt_br = "Exportação", en = "Export";
    PrefsExportWorkers: pt_br = "Workers de exportação em segundo plano", en = "Background export workers";
    PrefsOutputFolder: pt_br = "Pasta de saída padrão", en = "Default output folder";
    Browse: pt_br = "Procurar", en = "Browse";
    PrefsProject: pt_br = "Projeto", en = "Project";
    PrefsAutosaveInterval: pt_br = "Intervalo de autosave", en = "Autosave interval";
    PrefsShortcuts: pt_br = "Atalhos de teclado", en = "Keyboard shortcuts";
    TableAction: pt_br = "Ação", en = "Action";
    TableShortcut: pt_br = "Atalho", en = "Shortcut";
    ShortcutSplit: pt_br = "Dividir clipe (split)", en = "Split clip";
    ShortcutCut: pt_br = "Cortar", en = "Cut";
    ShortcutPlayPause: pt_br = "Play / Pause", en = "Play / Pause";
    ShortcutMarkInOut: pt_br = "Marcar entrada / saída", en = "Mark in / out";
    ShortcutSendToQueue: pt_br = "Enviar para fila de exportação", en = "Send to export queue";
    KeySpace: pt_br = "Espaço", en = "Space";
}

/// The breadcrumb's title for `screen` (e.g. `Screen::Queue` -> "Fila de exportação" — longer
/// than its [`nav_label`], which is space-constrained in the rail).
pub fn screen_title(locale: Locale, screen: Screen) -> &'static str {
    match screen {
        Screen::Home => Text::ScreenTitleHome.tr(locale),
        Screen::Editor => Text::ScreenTitleEditor.tr(locale),
        Screen::Library => Text::ScreenTitleLibrary.tr(locale),
        Screen::Queue => Text::ScreenTitleQueue.tr(locale),
        Screen::Prefs => Text::ScreenTitlePrefs.tr(locale),
    }
}

/// The nav rail's short label for `screen` (e.g. `Screen::Queue` -> "Fila").
pub fn nav_label(locale: Locale, screen: Screen) -> &'static str {
    match screen {
        Screen::Home => Text::NavHome.tr(locale),
        Screen::Editor => Text::NavEditor.tr(locale),
        Screen::Library => Text::NavLibrary.tr(locale),
        Screen::Queue => Text::NavQueue.tr(locale),
        Screen::Prefs => Text::NavPrefs.tr(locale),
    }
}

/// Formats a project's "last edited" recency (e.g. "Edited 2 hours ago").
pub fn recency_label(locale: Locale, recency: Recency) -> String {
    match (locale, recency) {
        (Locale::PtBr, Recency::HoursAgo(1)) => "Editado há 1 hora".to_string(),
        (Locale::PtBr, Recency::HoursAgo(n)) => format!("Editado há {n} horas"),
        (Locale::PtBr, Recency::Yesterday) => "Editado ontem".to_string(),
        (Locale::PtBr, Recency::DaysAgo(1)) => "Editado há 1 dia".to_string(),
        (Locale::PtBr, Recency::DaysAgo(n)) => format!("Editado há {n} dias"),
        (Locale::En, Recency::HoursAgo(1)) => "Edited 1 hour ago".to_string(),
        (Locale::En, Recency::HoursAgo(n)) => format!("Edited {n} hours ago"),
        (Locale::En, Recency::Yesterday) => "Edited yesterday".to_string(),
        (Locale::En, Recency::DaysAgo(1)) => "Edited 1 day ago".to_string(),
        (Locale::En, Recency::DaysAgo(n)) => format!("Edited {n} days ago"),
    }
}

/// Formats a project's clip/track kicker (e.g. "4 clips · V1/A1/A2").
pub fn track_summary(locale: Locale, clip_count: usize, track_names: &[String]) -> String {
    let unit = if clip_count == 1 {
        Text::ClipUnitSingular.tr(locale)
    } else {
        Text::ClipsUnit.tr(locale)
    };
    format!("{clip_count} {unit} · {}", track_names.join("/"))
}

/// The export queue's status pill text for `status` (e.g. "Renderizando", "Falhou").
pub fn job_status_label(locale: Locale, status: &ExportJobStatus) -> &'static str {
    match status {
        ExportJobStatus::Queued => Text::StatusQueued.tr(locale),
        ExportJobStatus::Rendering { .. } => Text::StatusRendering.tr(locale),
        ExportJobStatus::Paused { .. } => Text::StatusPaused.tr(locale),
        ExportJobStatus::Done => Text::StatusDone.tr(locale),
        ExportJobStatus::Failed { .. } => Text::StatusFailed.tr(locale),
    }
}

/// The export queue's secondary line for `job` — `"-14 LUFS · 42 Mbps · /export/…"` for a
/// normal job, or `"Erro: <message>"`/`"Error: <message>"` for a failed one.
pub fn job_detail_line(locale: Locale, job: &ExportJob) -> String {
    match &job.status {
        ExportJobStatus::Failed { message } => {
            format!("{}: {message}", Text::ErrorPrefix.tr(locale))
        }
        _ => format!(
            "-{:.0} LUFS · {:.0} Mbps · {}",
            -job.target_lufs, job.bitrate_mbps, job.output_path
        ),
    }
}

#[cfg(test)]
#[path = "i18n/i18n_test.rs"]
mod tests;
