// Copyright (C) 2026 by Lucas Gomes <lucasgsm88@gmail.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

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

/// A language the UI can be displayed in. Stored on [`crate::app::App`] and switched
/// at runtime from the Ajustes screen — nothing here requires a restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Locale {
    #[default]
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

    /// A short locale-code label (e.g. "PT-BR") for compact chrome like the breadcrumb's
    /// locale indicator, matching `oca-editor-mock.html`'s `.bc-locale` — `native_name` is too
    /// wide for that spot.
    pub fn short_code(self) -> &'static str {
        match self {
            Locale::PtBr => "PT-BR",
            Locale::En => "EN",
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

    WindowMinimize: pt_br = "Minimizar", en = "Minimize";
    WindowMaximize: pt_br = "Maximizar", en = "Maximize";
    WindowRestore: pt_br = "Restaurar", en = "Restore";
    WindowClose: pt_br = "Fechar", en = "Close";

    NavHome: pt_br = "Início", en = "Home";
    NavEditor: pt_br = "Editor", en = "Editor";
    NavLibrary: pt_br = "Mídia", en = "Media";
    NavSoundLibrary: pt_br = "Música/SFX", en = "Music/SFX";
    NavQueue: pt_br = "Fila", en = "Queue";
    NavPrefs: pt_br = "Ajustes", en = "Settings";

    ScreenTitleHome: pt_br = "Início", en = "Home";
    ScreenTitleEditor: pt_br = "Editor", en = "Editor";
    ScreenTitleLibrary: pt_br = "Mídia", en = "Media";
    ScreenTitleSoundLibrary: pt_br = "Música e efeitos sonoros", en = "Music & sound effects";
    ScreenTitleQueue: pt_br = "Fila de exportação", en = "Export queue";
    ScreenTitlePrefs: pt_br = "Ajustes", en = "Settings";
    UnsavedChanges: pt_br = "Alterações não salvas", en = "Unsaved changes";

    HomeTitle: pt_br = "Projetos recentes", en = "Recent projects";
    HomeSubtitle: pt_br = "Continue de onde parou ou comece um projeto novo.", en = "Continue where you left off or start a new project.";
    UpdateAvailable: pt_br = "Nova versão disponível:", en = "New version available:";
    UpdateAvailableLink: pt_br = "Ver no GitHub →", en = "View on GitHub →";
    AboutOpen: pt_br = "Sobre o oca", en = "About oca";
    AboutDescription: pt_br = "Editor de vídeo nativo", en = "Native video editor";
    AboutInstalledVersion: pt_br = "Versão instalada:", en = "Installed version:";
    AboutCheckingUpdates: pt_br = "Verificando atualizações...", en = "Checking for updates...";
    AboutUpToDate: pt_br = "Você está usando a versão mais recente.", en = "You are running the latest version.";
    AboutCheckFailed: pt_br = "Não foi possível verificar atualizações.", en = "Could not check for updates.";
    AboutDownloadInstall: pt_br = "Baixar e instalar", en = "Download and install";
    AboutInstalling: pt_br = "Baixando e instalando a atualização...", en = "Downloading and installing the update...";
    AboutRestartRequired: pt_br = "Atualização instalada. Reinicie para usar a versão", en = "Update installed. Restart to use version";
    AboutUpdateReadyToast: pt_br = "Atualização instalada. Abra Sobre o oca para reiniciar.", en = "Update installed. Open About oca to restart.";
    AboutRestartNow: pt_br = "Reiniciar agora", en = "Restart now";
    AboutInstallFailed: pt_br = "Não foi possível instalar a atualização.", en = "Could not install the update.";
    AboutRetryInstall: pt_br = "Tentar novamente", en = "Try again";
    AboutManualInstallOnly: pt_br = "A instalação automática requer o pacote AppImage no Linux. Baixe o pacote completo desta versão.", en = "Automatic installation requires the Linux AppImage package. Download this version's complete bundle.";
    AboutRestartFailed: pt_br = "Não foi possível reiniciar o aplicativo.", en = "Could not restart the application.";
    AboutViewReleases: pt_br = "Ver releases no GitHub →", en = "View GitHub releases →";
    NewProject: pt_br = "＋ Novo projeto", en = "＋ New project";
    OpenProject: pt_br = "Abrir projeto…", en = "Open project…";
    UntitledProject: pt_br = "Projeto sem título", en = "Untitled project";
    HomeCtxRemove: pt_br = "Remover da lista", en = "Remove from list";
    HomeCtxShowInFinder: pt_br = "Mostrar no Finder", en = "Show in Finder";
    Save: pt_br = "💾 Salvar", en = "💾 Save";
    ClipsUnit: pt_br = "clipes", en = "clips";
    ClipUnitSingular: pt_br = "clipe", en = "clip";

    MenuFile: pt_br = "Arquivo", en = "File";
    MenuEdit: pt_br = "Editar", en = "Edit";
    MenuView: pt_br = "Ver", en = "View";
    MenuSequence: pt_br = "Sequência", en = "Sequence";
    MenuClip: pt_br = "Clipe", en = "Clip";
    MenuMarkers: pt_br = "Marcadores", en = "Markers";
    MenuGraphics: pt_br = "Gráficos", en = "Graphics";
    MenuHelp: pt_br = "Ajuda", en = "Help";
    MenuHelpAbout: pt_br = "Sobre o oca...", en = "About oca...";
    MenuSequenceAddTab: pt_br = "Nova sequência", en = "New sequence";
    MenuSequenceRename: pt_br = "Renomear...", en = "Rename...";
    MenuSequenceDuplicate: pt_br = "Duplicar", en = "Duplicate";
    MenuSequenceMoveLeft: pt_br = "Mover para esquerda", en = "Move left";
    MenuSequenceMoveRight: pt_br = "Mover para direita", en = "Move right";
    MenuSequenceDelete: pt_br = "Excluir...", en = "Delete...";
    MenuSequenceInsertAsCompoundClip: pt_br = "📦 Inserir sequência como clipe composto", en = "📦 Insert sequence as compound clip";
    MenuSequenceInsertAsCompoundClipEmpty: pt_br = "Nenhuma outra sequência disponível", en = "No other sequence available";
    MenuClipMode: pt_br = "Modo de edição", en = "Edit mode";
    ToolSelect: pt_br = "Selecionar", en = "Select";
    ToolCut: pt_br = "Cortar / Split", en = "Cut / Split";
    ToolTrim: pt_br = "Aparar", en = "Trim";
    ToolRipple: pt_br = "Ripple", en = "Ripple";
    ToolRoll: pt_br = "Roll", en = "Roll";
    ToolSlip: pt_br = "Slip", en = "Slip";
    ToolSlide: pt_br = "Slide", en = "Slide";
    ToolHand: pt_br = "Mão (pan)", en = "Hand (pan)";
    ToolRazor: pt_br = "Navalha", en = "Razor";
    ToolText: pt_br = "Texto", en = "Text";
    ToolEffects: pt_br = "Efeitos", en = "Effects";
    ToolZoomTool: pt_br = "Zoom", en = "Zoom";
    EffectsPanelTitle: pt_br = "Efeitos", en = "Effects";
    EffectsPanelNoSelectionHint: pt_br = "Selecione um clipe para aplicar um efeito.", en = "Select a clip to apply an effect.";
    EffectCategoryBlurSharpen: pt_br = "Desfoque e Nitidez", en = "Blur & Sharpen";
    EffectCategoryColor: pt_br = "Cor", en = "Color";
    EffectCategoryDistortion: pt_br = "Distorção", en = "Distortion";
    EffectCategoryKeying: pt_br = "Chroma Key", en = "Keying";
    EffectCategoryUtility: pt_br = "Utilitário", en = "Utility";
    EffectBlur: pt_br = "Desfoque", en = "Blur";
    EffectSharpen: pt_br = "Nitidez", en = "Sharpen";
    EffectBlackAndWhite: pt_br = "Preto e Branco", en = "Black & White";
    EffectSepia: pt_br = "Sépia", en = "Sepia";
    EffectVignette: pt_br = "Vinheta", en = "Vignette";
    EffectGlitch: pt_br = "Glitch", en = "Glitch";
    EffectPixelize: pt_br = "Pixelizar", en = "Pixelize";
    EffectShake: pt_br = "Tremido", en = "Shake";
    EffectChromaKey: pt_br = "Chroma Key", en = "Chroma Key";
    EffectBackgroundRemoval: pt_br = "Remover Fundo", en = "Background Removal";
    EffectStabilization: pt_br = "Estabilização", en = "Stabilization";
    EffectFreeze: pt_br = "Congelar", en = "Freeze";
    EffectDeflicker: pt_br = "Deflicker", en = "Deflicker";
    ExportElapsed: pt_br = "decorrido", en = "elapsed";
    TrimInfoSource: pt_br = "Fonte", en = "Source";
    TrimInfoSequence: pt_br = "Sequência", en = "Sequence";
    TrimInfoDuration: pt_br = "Duração", en = "Duration";
    Export: pt_br = "⭳ Exportar", en = "⭳ Export";
    ExportSrt: pt_br = "Exportar .srt", en = "Export .srt";
    ExportSrtHint: pt_br = "Salva as legendas da timeline como um arquivo .srt separado.", en = "Saves the timeline's subtitles as a separate .srt file.";
    ExportSrtEmpty: pt_br = "Nenhuma legenda na timeline para exportar.", en = "No subtitles on the timeline to export.";
    MediaLibrary: pt_br = "Biblioteca de mídia", en = "Media library";
    SearchMediaPlaceholder: pt_br = "Buscar mídia…", en = "Search media…";
    MediaViewList: pt_br = "Visualização em lista", en = "List view";
    MediaViewGrid: pt_br = "Visualização em grade", en = "Grid view";
    TimelineZoom: pt_br = "Zoom", en = "Zoom";
    TimelineZoomIn: pt_br = "Aumentar zoom", en = "Zoom in";
    TimelineZoomOut: pt_br = "Diminuir zoom", en = "Zoom out";
    SelectedClip: pt_br = "Clipe selecionado", en = "Selected clip";
    NoClipSelected: pt_br = "Nenhum clipe selecionado", en = "No clip selected";
    PropertiesTabInspector: pt_br = "Inspetor", en = "Inspector";
    PropertiesTabEffects: pt_br = "Efeitos", en = "Effects";
    PropertiesTabAudio: pt_br = "Áudio", en = "Audio";
    PropCodec: pt_br = "Codec", en = "Codec";
    PropSourceBitrate: pt_br = "Bitrate fonte", en = "Source bitrate";
    PropResolution: pt_br = "Resolução", en = "Resolution";
    PropFps: pt_br = "FPS", en = "FPS";
    PropLoudness: pt_br = "Loudness", en = "Loudness";
    PropProxy: pt_br = "Proxy de edição", en = "Editing proxy";
    ProxyPresent: pt_br = "540p pronto", en = "540p ready";
    ProxyAbsent: pt_br = "usando original", en = "using original";
    PropGain: pt_br = "Ganho do bloco", en = "Block gain";
    GainExportNote: pt_br = "aplicado na exportação (filtro volume) — sem efeito no preview ao vivo.", en = "applied on export (volume filter) — no live preview effect.";
    PropGainKeyframes: pt_br = "Ganho (keyframes)", en = "Gain (keyframes)";
    GainKeyframesExportNote: pt_br = "quando definido, substitui o ganho constante acima na exportação (fade/rampa de volume) — sem efeito no preview ao vivo.", en = "when set, overrides the constant gain above on export (volume fade/ramp) — no live preview effect.";
    PropBrightnessKeyframes: pt_br = "Brilho (keyframes)", en = "Brightness (keyframes)";
    PropContrastKeyframes: pt_br = "Contraste (keyframes)", en = "Contrast (keyframes)";
    PropSaturationKeyframes: pt_br = "Saturação (keyframes)", en = "Saturation (keyframes)";
    ColorKeyframesExportNote: pt_br = "quando definido, substitui o valor constante desse eixo na exportação — sem efeito no preview ao vivo.", en = "when set, overrides this axis's constant value on export — no live preview effect.";
    PropCropXKeyframes: pt_br = "Recorte X (keyframes)", en = "Crop X (keyframes)";
    PropCropYKeyframes: pt_br = "Recorte Y (keyframes)", en = "Crop Y (keyframes)";
    PropCropWKeyframes: pt_br = "Recorte largura (keyframes)", en = "Crop width (keyframes)";
    PropCropHKeyframes: pt_br = "Recorte altura (keyframes)", en = "Crop height (keyframes)";
    CropKeyframesExportNote: pt_br = "quando definido, substitui o valor constante desse eixo na exportação (recorte/panorâmica animados) — sem efeito no preview ao vivo.", en = "when set, overrides this axis's constant value on export (animated crop/pan) — no live preview effect.";
    PropFreeze: pt_br = "❄ Congelar quadro", en = "❄ Freeze frame";
    FreezeExportNote: pt_br = "aplicado na exportação — sem efeito no preview ao vivo.", en = "applied on export — no live preview effect.";
    PropDeflicker: pt_br = "✨ Remover flicker", en = "✨ Remove flicker";
    DeflickerExportNote: pt_br = "aplicado no export — preview ainda não mostra.", en = "applied on export — preview doesn't show it yet.";
    PropSpeed: pt_br = "Velocidade", en = "Speed";
    SpeedExportNote: pt_br = "aplicado na exportação (vídeo e áudio) — sem efeito no preview ao vivo.", en = "applied on export (video and audio) — no live preview effect.";
    PropCrop: pt_br = "Recorte (x, y, largura, altura)", en = "Crop (x, y, width, height)";
    CropReset: pt_br = "Redefinir recorte", en = "Reset crop";
    CropExportNote: pt_br = "aplicado na exportação — sem efeito no preview ao vivo.", en = "applied on export — no live preview effect.";
    PropMask: pt_br = "Máscara", en = "Mask";
    MaskNone: pt_br = "Nenhuma", en = "None";
    MaskCircle: pt_br = "Círculo", en = "Circle";
    MaskRoundedRect: pt_br = "Retângulo arredondado", en = "Rounded rectangle";
    MaskCornerRadius: pt_br = "Raio do canto", en = "Corner radius";
    MaskExportNote: pt_br = "aplicado na exportação, mas só tem efeito em blocos de faixas de overlay — sem efeito no preview ao vivo.", en = "applied on export, but only has an effect on overlay-track blocks — no live preview effect.";
    PropFlip: pt_br = "⇄ Espelhar horizontal", en = "⇄ Flip horizontal";
    FlipExportNote: pt_br = "aplicado na exportação — sem efeito no preview ao vivo.", en = "applied on export — no live preview effect.";
    PropColorFilter: pt_br = "Filtro de cor", en = "Color filter";
    ColorFilterNone: pt_br = "Nenhum", en = "None";
    ColorFilterBlackAndWhite: pt_br = "Preto e branco", en = "Black and white";
    ColorFilterSepia: pt_br = "Sépia", en = "Sepia";
    ColorFilterExportNote: pt_br = "aplicado na exportação — sem efeito no preview ao vivo.", en = "applied on export — no live preview effect.";
    PropBlendMode: pt_br = "Modo de mesclagem", en = "Blend mode";
    BlendModeExportNote: pt_br = "só afeta trilhas de overlay; ignora posicionamento (PIP) enquanto ativo.", en = "only affects overlay tracks; ignores positioning (PIP) while active.";
    PropLut: pt_br = "LUT 3D", en = "3D LUT";
    LutExportNote: pt_br = "aplicado na exportação via lut3d — sem suporte no preview ao vivo (nenhum elemento de LUT disponível na instalação do GStreamer).", en = "applied on export via lut3d — no live preview support (no LUT element available in the GStreamer install).";
    ClearLut: pt_br = "Remover", en = "Clear";
    PropLayerSize: pt_br = "Tamanho da camada", en = "Layer size";
    LayerSizeExportNote: pt_br = "redimensiona a camada no canvas — só tem efeito em faixas de overlay.", en = "resizes the layer on the canvas — only has an effect on overlay tracks.";
    PropLayerWidth: pt_br = "Largura", en = "Width";
    PropLayerHeight: pt_br = "Altura", en = "Height";
    PropVignette: pt_br = "Vinheta", en = "Vignette";
    VignetteExportNote: pt_br = "aplicado na exportação — sem efeito no preview ao vivo.", en = "applied on export — no live preview effect.";
    PropColorAdjust: pt_br = "Cor", en = "Color";
    PropBrightness: pt_br = "Brilho", en = "Brightness";
    PropContrast: pt_br = "Contraste", en = "Contrast";
    PropSaturation: pt_br = "Saturação", en = "Saturation";
    ColorAdjustExportNote: pt_br = "aplicado na exportação (filtro eq) — sem efeito no preview ao vivo.", en = "applied on export (eq filter) — no live preview effect.";
    PropSharpen: pt_br = "Nitidez", en = "Sharpen";
    SharpenExportNote: pt_br = "aplicado na exportação — sem efeito no preview ao vivo.", en = "applied on export — no live preview effect.";
    PropChromaKey: pt_br = "🟩 Chroma key", en = "🟩 Chroma key";
    ChromaKeyColor: pt_br = "Cor:", en = "Color:";
    ChromaKeyTolerance: pt_br = "Tolerância", en = "Tolerance";
    ChromaKeyExportNote: pt_br = "aplicado na exportação, mas só tem efeito em blocos de faixas de overlay — sem efeito no preview ao vivo.", en = "applied on export, but only has an effect on overlay-track blocks — no live preview effect.";
    PropVoiceCleanup: pt_br = "🎤 Limpeza de voz", en = "🎤 Voice cleanup";
    VoiceCleanupNoiseFloor: pt_br = "Piso de ruído", en = "Noise floor";
    VoiceCleanupCompressorThreshold: pt_br = "Limiar do compressor", en = "Compressor threshold";
    VoiceCleanupCompressorRatio: pt_br = "Proporção do compressor", en = "Compressor ratio";
    VoiceCleanupCeiling: pt_br = "Teto do limitador", en = "Limiter ceiling";
    VoiceCleanupExportNote: pt_br = "aplicado na exportação (cadeia de redução de ruído + compressor + limitador) — sem efeito no preview ao vivo.", en = "applied on export (noise-reduction + compressor + limiter chain) — no live preview effect.";
    VoiceCleanupMicRoleSuggestion: pt_br = "Este clipe está numa faixa com papel \"Microfone\" — considere ativar a limpeza de voz.", en = "This clip is on a track with the \"Mic\" role — consider enabling voice cleanup.";
    VoiceCleanupPreviewButton: pt_br = "🔊 Prévia A/B", en = "🔊 A/B preview";
    VoiceCleanupPreviewRendering: pt_br = "Renderizando prévia…", en = "Rendering preview…";
    VoiceCleanupPreviewPlayOriginal: pt_br = "▶ Original", en = "▶ Original";
    VoiceCleanupPreviewPlayProcessed: pt_br = "▶ Tratado", en = "▶ Processed";
    VoiceCleanupPreviewFailed: pt_br = "Falha ao renderizar a prévia", en = "Failed to render preview";
    VoiceCleanupPreviewPlaybackFailed: pt_br = "Falha ao reproduzir a prévia", en = "Failed to play preview";
    PropBackgroundRemoval: pt_br = "🤖 Remoção de fundo (IA)", en = "🤖 Background removal (AI)";
    BackgroundRemovalExportNote: pt_br = "afeta a exportação só em blocos de faixas de overlay (não na faixa de fundo) — clique em \"Gerar máscara\" antes de exportar.", en = "only affects export on overlay-track blocks (not the background track) — click \"Generate matte\" before exporting.";
    BackgroundRemovalGenerateMatte: pt_br = "Gerar máscara", en = "Generate matte";
    BackgroundRemovalGenerating: pt_br = "Gerando máscara...", en = "Generating matte...";
    BackgroundRemovalNoModelConfigured: pt_br = "O modelo de remoção de fundo não está disponível no pacote.", en = "The background removal model is missing from the application bundle.";
    PropPrivacyBlur: pt_br = "🕶 Blur de privacidade", en = "🕶 Privacy blur";
    PrivacyBlurExportNote: pt_br = "borra a região selecionada durante toda a duração deste bloco na exportação — clique em \"Aplicar blur\" após ajustar a região.", en = "blurs the selected region for this block's whole on-timeline duration in export — click \"Apply blur\" after adjusting the region.";
    PrivacyBlurSigma: pt_br = "Intensidade do blur", en = "Blur intensity";
    PrivacyBlurRegionWidth: pt_br = "Largura da região", en = "Region width";
    PrivacyBlurRegionHeight: pt_br = "Altura da região", en = "Region height";
    PrivacyBlurRegionReset: pt_br = "Centralizar região", en = "Center region";
    PrivacyBlurApply: pt_br = "Aplicar blur", en = "Apply blur";
    PrivacyBlurGenerating: pt_br = "Aplicando blur...", en = "Applying blur...";
    PrefsBackgroundRemovalModelPath: pt_br = "Caminho do modelo de remoção de fundo", en = "Background removal model path";
    TtsButton: pt_br = "🔊 Texto-pra-fala", en = "🔊 Text-to-speech";
    TtsModalTitle: pt_br = "Texto-pra-fala", en = "Text-to-speech";
    TtsGenerate: pt_br = "Gerar", en = "Generate";
    TtsNoModelConfigured: pt_br = "O modelo de voz não está disponível no pacote.", en = "The voice model is missing from the application bundle.";
    TtsGenerationFailed: pt_br = "Falha ao gerar narração", en = "Narration generation failed";
    TtsGenerating: pt_br = "Gerando narração…", en = "Generating narration…";
    YoutubeDownloadButton: pt_br = "⭳ Baixar do YouTube", en = "⭳ Download from YouTube";
    YoutubeDownloadModalTitle: pt_br = "Baixar do YouTube", en = "Download from YouTube";
    YoutubeDownloadUrlHint: pt_br = "Link do vídeo", en = "Video URL";
    YoutubeDownloadFormatMp4: pt_br = "MP4 (vídeo)", en = "MP4 (video)";
    YoutubeDownloadFormatMp3: pt_br = "MP3 (áudio)", en = "MP3 (audio)";
    YoutubeDownloadQuality: pt_br = "Qualidade", en = "Quality";
    YoutubeDownloadStart: pt_br = "Baixar", en = "Download";
    YoutubeDownloadCancel: pt_br = "Cancelar", en = "Cancel";
    YoutubeDownloadInProgress: pt_br = "Baixando…", en = "Downloading…";
    YoutubeDownloadToolMissing: pt_br = "Componente de download do YouTube não encontrado — reinstale o aplicativo.", en = "YouTube download component not found — reinstall the app.";
    PrefsTtsModelPath: pt_br = "Caminho do modelo de texto-pra-fala", en = "Text-to-speech model path";
    PropOtherEffects: pt_br = "Outros efeitos", en = "Other effects";
    PropBlur: pt_br = "Blur", en = "Blur";
    PropShake: pt_br = "Tremido", en = "Shake";
    PropGlitch: pt_br = "Glitch", en = "Glitch";
    PropPixelize: pt_br = "Pixelizar", en = "Pixelize";
    OtherEffectsExportNote: pt_br = "aplicado na exportação — tremido e pixelizar também têm efeito no preview ao vivo; blur e glitch, não.", en = "applied on export — shake and pixelize also affect the live preview; blur and glitch don't.";
    PropStabilization: pt_br = "🎥 Estabilização", en = "🎥 Stabilization";
    StabilizationExportNote: pt_br = "aplicado na exportação via deshake — sem suporte no preview ao vivo (nenhum elemento de estabilização disponível na instalação do GStreamer).", en = "applied on export via deshake — no live preview support (no stabilization element available in the GStreamer install).";
    PropTransition: pt_br = "Transição", en = "Transition";
    TransitionNone: pt_br = "Nenhuma", en = "None";
    TransitionFade: pt_br = "Fade", en = "Fade";
    TransitionHardCut: pt_br = "Corte seco", en = "Hard cut";
    TransitionSlide: pt_br = "Slide", en = "Slide";
    TransitionZoom: pt_br = "Zoom", en = "Zoom";
    PropTransitionDuration: pt_br = "Duração da transição", en = "Transition duration";
    TransitionExportNote: pt_br = "aplicado na exportação — sem efeito no preview ao vivo. Modela só a transição de entrada deste bloco, não uma mistura real entre dois clipes.", en = "applied on export — no live preview effect. Models only this block's incoming transition, not a real cross-blend between two clips.";
    PropPositionKeyframes: pt_br = "Posição (keyframes)", en = "Position (keyframes)";
    PropScaleKeyframes: pt_br = "Escala (keyframes / punch-in)", en = "Scale (keyframes / punch-in)";
    PropRotationKeyframes: pt_br = "Rotação (keyframes)", en = "Rotation (keyframes)";
    PropOpacityKeyframes: pt_br = "Opacidade (keyframes)", en = "Opacity (keyframes)";
    PropAnchor: pt_br = "Ponto de ancoragem", en = "Anchor point";
    AddKeyframe: pt_br = "+ Adicionar keyframe", en = "+ Add keyframe";
    RemoveKeyframe: pt_br = "Remover keyframe", en = "Remove keyframe";
    RemoveVertex: pt_br = "Remover vértice", en = "Remove vertex";
    KeyframeTime: pt_br = "t", en = "t";
    KeyframeX: pt_br = "x", en = "x";
    KeyframeY: pt_br = "y", en = "y";
    LayerTransformDragHint: pt_br = "Arraste para posicionar a camada — efeito visível só em faixas de overlay.", en = "Drag to position the layer — only visible on overlay tracks.";
    LayerTransformAnimatedHint: pt_br = "Posição animada — edite pela lista de keyframes.", en = "Position is animated — edit via the keyframe list.";
    LayerTransformResizeHint: pt_br = "Arraste para redimensionar a camada — efeito visível só em faixas de overlay.", en = "Drag to resize the layer — only visible on overlay tracks.";
    PositionExportNote: pt_br = "aplicado na exportação apenas em clipes de faixa de sobreposição; sem efeito no preview ao vivo ainda.", en = "applied on export only for overlay-track clips; no live preview effect yet.";
    ScaleExportNote: pt_br = "aplicado na exportação e no preview ao vivo.", en = "applied on export and live preview.";
    RotationExportNote: pt_br = "aplicado na exportação; sem efeito no preview ao vivo ainda.", en = "applied on export; no live preview effect yet.";
    OpacityExportNote: pt_br = "aplicado na exportação apenas em clipes de faixa de sobreposição; sem efeito no preview ao vivo ainda.", en = "applied on export only for overlay-track clips; no live preview effect yet.";
    AnchorExportNote: pt_br = "ponto ao redor do qual rotação e escala giram/crescem — aplicado na exportação; sem efeito no preview ao vivo ainda.", en = "point rotation and scale pivot/grow around — applied on export; no live preview effect yet.";
    OnExport: pt_br = "Ao exportar", en = "On export";
    NormalizeTo: pt_br = "Normalizar", en = "Normalize";
    BitrateFromSource: pt_br = "Bitrate = fonte", en = "Bitrate = source";
    ExportAutoNote: pt_br = "redução de ruído + loudnorm 2-pass + true peak limiter, aplicado automático no export — sem ajuste manual por clipe.", en = "noise reduction + loudnorm 2-pass + true peak limiter, applied automatically on export — no per-clip manual adjustment.";
    Timeline: pt_br = "TIMELINE", en = "TIMELINE";
    TimelineEmpty: pt_br = "Este projeto ainda não tem clipes na timeline.", en = "This project doesn't have any clips on the timeline yet.";
    ContextMenuSplit: pt_br = "✂ Dividir no playhead", en = "✂ Split at playhead";
    ContextMenuDelete: pt_br = "🗑 Excluir", en = "🗑 Delete";
    ContextMenuColorLabel: pt_br = "🎨 Rótulo de cor", en = "🎨 Color label";
    ContextMenuColorLabelClear: pt_br = "Limpar rótulo", en = "Clear label";
    ContextMenuDetachAudio: pt_br = "🎧 Destacar áudio", en = "🎧 Detach audio";
    ContextMenuSpeedRamp: pt_br = "⏱ Rampa de velocidade", en = "⏱ Speed ramp";
    SpeedRampSlowToFast: pt_br = "Lento → Rápido (0.5x → 2x)", en = "Slow → Fast (0.5x → 2x)";
    SpeedRampFastToSlow: pt_br = "Rápido → Lento (2x → 0.5x)", en = "Fast → Slow (2x → 0.5x)";
    SpeedRampCustom: pt_br = "Personalizada…", en = "Custom…";
    SpeedRampCustomTitle: pt_br = "Rampa de velocidade personalizada", en = "Custom speed ramp";
    SpeedRampStartSpeedLabel: pt_br = "Velocidade inicial", en = "Start speed";
    SpeedRampEndSpeedLabel: pt_br = "Velocidade final", en = "End speed";
    SpeedRampStepsLabel: pt_br = "Número de etapas", en = "Step count";
    SpeedRampStepsHint: pt_br = "mínimo 2", en = "minimum 2";
    SpeedRampSmoothToggle: pt_br = "Curva suave e contínua (em vez de degraus)", en = "Smooth, continuous curve (instead of steps)";
    ContextMenuCreateCompoundClip: pt_br = "📦 Criar clipe composto", en = "📦 Create compound clip";
    ContextMenuOpenCompoundClip: pt_br = "📦 Abrir clipe composto", en = "📦 Open compound clip";
    SpeedRampApply: pt_br = "Aplicar rampa", en = "Apply ramp";
    ContextMenuCopy: pt_br = "⧉ Copiar", en = "⧉ Copy";
    ContextMenuCut: pt_br = "✂ Recortar", en = "✂ Cut";
    ContextMenuPaste: pt_br = "📋 Colar no playhead", en = "📋 Paste at playhead";
    ContextMenuCopyFormatting: pt_br = "🎨 Copiar formatação", en = "🎨 Copy formatting";
    ContextMenuPasteFormatting: pt_br = "🎨 Colar formatação", en = "🎨 Paste formatting";
    DefaultSequenceName: pt_br = "Sequência principal", en = "Main sequence";
    AddSequenceTab: pt_br = "＋", en = "＋";
    MergeIntoComposite: pt_br = "⛓ Mesclar em bloco", en = "⛓ Merge into block";
    MergeIntoCompositeHint: pt_br = "Ctrl+clique em 2+ clipes na timeline pra selecionar, depois mescle num bloco composto", en = "Ctrl+click 2+ clips on the timeline to pick them, then merge into a composite block";
    SaveAsTemplate: pt_br = "🗂 Salvar como template", en = "🗂 Save as template";
    SaveAsTemplateHint: pt_br = "Ctrl+clique 1+ clipe na timeline, depois salve o grupo como template reaproveitável", en = "Ctrl+click 1+ clip on the timeline, then save the group as a reusable template";
    Templates: pt_br = "📋 Templates", en = "📋 Templates";
    SaveTemplateTitle: pt_br = "Salvar template", en = "Save template";
    TemplateNameLabel: pt_br = "Nome do template", en = "Template name";
    SaveTemplateConfirm: pt_br = "Salvar", en = "Save";
    NoSavedTemplates: pt_br = "Nenhum template salvo ainda.", en = "No saved templates yet.";
    ApplyTemplate: pt_br = "Aplicar", en = "Apply";
    DeleteTemplate: pt_br = "🗑", en = "🗑";
    ApplyTemplateTitle: pt_br = "Aplicar template", en = "Apply template";
    ApplyTemplateLayerLabel: pt_br = "Camada", en = "Layer";
    ApplyTemplatePickAsset: pt_br = "Escolha um arquivo…", en = "Pick a file…";
    ApplyTemplateConfirm: pt_br = "Criar camadas", en = "Create layers";
    GraphicTemplateApplyInvalid: pt_br = "Não foi possível aplicar o template gráfico", en = "Could not apply the graphic template";
    LoadGraphicTemplate: pt_br = "🖼 Carregar template gráfico", en = "🖼 Load graphic template";
    GraphicTemplateReadFailed: pt_br = "Não foi possível ler o arquivo de template", en = "Could not read the template file";
    GraphicTemplateApplyTitle: pt_br = "Preencher template", en = "Fill in template";
    GraphicTemplateApplyConfirm: pt_br = "Aplicar", en = "Apply";
    TrackKindVideo: pt_br = "Vídeo", en = "Video";
    TrackKindAudio: pt_br = "Áudio", en = "Audio";
    TrackKindText: pt_br = "Texto", en = "Text";
    TrackKindShape: pt_br = "Forma", en = "Shape";
    PreviewUnavailable: pt_br = "Pré-visualização indisponível", en = "Preview unavailable";
    EnterFullscreenPreview: pt_br = "Tela cheia", en = "Fullscreen";
    ExitFullscreenPreview: pt_br = "Sair da tela cheia", en = "Exit fullscreen";

    LibraryTitle: pt_br = "Biblioteca de mídia", en = "Media library";
    ImportFiles: pt_br = "⭱ Importar arquivos", en = "⭱ Import files";
    Importing: pt_br = "Importando arquivos…", en = "Importing files…";
    LibraryEmpty: pt_br = "Nenhum arquivo importado neste projeto ainda.", en = "No files imported into this project yet.";
    DropFilesHint: pt_br = "Solte os arquivos aqui para importar", en = "Drop files here to import";
    ProxyReady: pt_br = "Proxy 540p", en = "540p proxy";
    TranscribeAction: pt_br = "Transcrever", en = "Transcribe";
    TranscribeInProgress: pt_br = "Transcrevendo...", en = "Transcribing...";
    TranscribeNoModelConfigured: pt_br = "O modelo Whisper não está disponível no pacote.", en = "The Whisper model is missing from the application bundle.";
    TranscribeNoSpeechFound: pt_br = "Nenhuma fala reconhecida no áudio.", en = "No speech recognized in the audio.";
    PrefsWhisperModelPath: pt_br = "Modelo Whisper (legendas automáticas)", en = "Whisper model (automatic subtitles)";
    PrefsSoundLibraryPath: pt_br = "Pasta da biblioteca de música/SFX", en = "Music/SFX library folder";
    PrefsReframeModelPath: pt_br = "Modelo de reenquadramento automático", en = "Auto-reframe model";
    ModelFile: pt_br = "Arquivo de modelo", en = "Model file";
    BundledResourceAvailable: pt_br = "Recurso local disponível (incluído no pacote ou override).", en = "Local resource available (bundled or override).";
    BundledResourceMissing: pt_br = "Recurso obrigatório ausente do pacote.", en = "Required resource is missing from the bundle.";

    AutoReframeAction: pt_br = "Reenquadramento automático", en = "Auto-reframe";
    AutoReframeInProgress: pt_br = "Reenquadrando...", en = "Reframing...";
    AutoReframeNoModelConfigured: pt_br = "O modelo de reenquadramento não está disponível no pacote.", en = "The auto-reframe model is missing from the application bundle.";
    AutoReframeNoSubjectFound: pt_br = "Nenhum rosto detectado — recorte centralizado aplicado.", en = "No face detected — applied a centered crop instead.";
    DynamicReframeAction: pt_br = "Reenquadramento dinâmico", en = "Dynamic reframe";
    DynamicReframeInProgress: pt_br = "Reenquadrando...", en = "Reframing...";
    DynamicReframeHint: pt_br = "Acompanha o sujeito ao longo do clipe, gerando keyframes de recorte em vez de um único enquadramento fixo.", en = "Tracks the subject across the clip, generating crop keyframes instead of a single fixed framing.";
    ReframeSeedPointToggle: pt_br = "Ancorar manualmente", en = "Manual anchor";
    ReframeSeedPointHint: pt_br = "Usa este ponto fixo como âncora do reenquadramento em vez de detectar rostos — nenhum modelo é necessário.", en = "Uses this fixed point as the reframe anchor instead of detecting faces — no model required.";

    MotionTrackAction: pt_br = "Rastrear movimento", en = "Track motion";
    MotionTrackInProgress: pt_br = "Rastreando...", en = "Tracking...";
    MotionTrackNoFramesDecoded: pt_br = "Não foi possível decodificar quadros suficientes para rastrear.", en = "Couldn't decode enough frames to track.";
    PropMotionTrackRegion: pt_br = "Região a rastrear", en = "Region to track";
    PropMotionTrackRegionHint: pt_br = "centro X/Y e tamanho, como fração do quadro de origem", en = "center X/Y and size, as a fraction of the source frame";
    PropMotionTrackWidth: pt_br = "Largura", en = "Width";
    PropMotionTrackHeight: pt_br = "Altura", en = "Height";
    PropMotionTrackSearchRadius: pt_br = "Raio de busca", en = "Search radius";
    MotionTrackRegionReset: pt_br = "Centralizar região", en = "Center region";
    MotionTrackRegionPick: pt_br = "🎯 Selecionar na prévia", en = "🎯 Pick in preview";
    MotionTrackRegionPickActive: pt_br = "🎯 Selecionando… (Esc p/ sair)", en = "🎯 Selecting… (Esc to exit)";
    MotionTrackRegionPickNeedsPreview: pt_br = "Carregue um clipe no preview antes de selecionar a região", en = "Load a clip in the preview before selecting the region";
    MotionTrackRegionPickHint: pt_br = "Arraste o bloco para mover · alça no canto redimensiona · Esc sai", en = "Drag the block to move it · corner handle resizes · Esc exits";

    SoundLibraryTitle: pt_br = "Música e efeitos sonoros", en = "Music & sound effects";
    SoundLibraryRescan: pt_br = "⟳ Atualizar", en = "⟳ Rescan";
    SoundLibraryMusic: pt_br = "Música", en = "Music";
    SoundLibrarySfx: pt_br = "Efeitos sonoros", en = "Sound effects";
    SoundLibraryAddToTimeline: pt_br = "+ Adicionar à timeline", en = "+ Add to timeline";
    SoundLibraryNoFolderConfigured: pt_br = "Nenhuma pasta configurada. Escolha uma pasta com subpastas \"music\"/\"sfx\" em Ajustes.", en = "No folder configured. Pick a folder with \"music\"/\"sfx\" subfolders in Preferences.";
    SoundLibraryEmpty: pt_br = "Nenhuma faixa encontrada. Adicione arquivos de áudio nas subpastas \"music\"/\"sfx\" da pasta configurada.", en = "No tracks found. Add audio files to the configured folder's \"music\"/\"sfx\" subfolders.";

    NavWatchFolder: pt_br = "Limpeza", en = "Cleanup";
    WatchFolderTitle: pt_br = "Limpeza de áudio por pasta", en = "Watched-folder audio cleanup";
    WatchFolderSubtitle: pt_br = "Monitora uma pasta e limpa automaticamente o áudio de gravações novas (redução de ruído + normalização de volume), sem tocar no vídeo.", en = "Watches a folder and automatically cleans up new recordings' audio (noise reduction + loudness normalization), without touching the video.";
    WatchFolderPathLabel: pt_br = "Pasta monitorada", en = "Watched folder";
    WatchFolderChooseFolder: pt_br = "Escolher pasta...", en = "Choose folder...";
    WatchFolderNoFolder: pt_br = "Escolha uma pasta para começar a monitorar.", en = "Choose a folder to start watching.";
    WatchFolderStart: pt_br = "▶ Iniciar monitoramento", en = "▶ Start watching";
    WatchFolderStop: pt_br = "■ Parar monitoramento", en = "■ Stop watching";
    WatchFolderOutputHint: pt_br = "Cópias limpas são salvas em \"processed\" dentro da pasta monitorada.", en = "Cleaned-up copies are saved to \"processed\" inside the watched folder.";
    WatchFolderEmpty: pt_br = "Nenhum arquivo detectado ainda.", en = "No files detected yet.";
    WatchFolderStatusStabilizing: pt_br = "Aguardando gravação terminar", en = "Waiting for the recording to finish";
    WatchFolderStatusMeasuringBefore: pt_br = "Analisando original", en = "Analyzing original";
    WatchFolderStatusProcessing: pt_br = "Processando", en = "Processing";
    WatchFolderStatusDone: pt_br = "Concluído", en = "Done";
    WatchFolderStatusError: pt_br = "Erro", en = "Error";
    WatchFolderLufsBeforeAfter: pt_br = "{before} → {after} LUFS", en = "{before} → {after} LUFS";
    WatchFolderAddToProject: pt_br = "+ Adicionar ao projeto", en = "+ Add to project";
    WatchFolderAddedToProject: pt_br = "✓ Adicionado ao projeto", en = "✓ Added to project";
    WatchFolderNeedsOpenProject: pt_br = "Abra um projeto para adicionar arquivos limpos a ele.", en = "Open a project to add cleaned-up files to it.";

    QueueTitle: pt_br = "Fila de exportação", en = "Export queue";
    AddExport: pt_br = "＋ Adicionar exportação", en = "＋ Add export";
    AddExportNeedsClip: pt_br = "Adicione ao menos um clipe de vídeo na sequência ativa primeiro", en = "Add at least one video clip to the active sequence first";
    AddVideoTrack: pt_br = "＋ Adicionar faixa de vídeo", en = "＋ Add video track";
    TrackHide: pt_br = "Ocultar faixa", en = "Hide track";
    TrackShow: pt_br = "Mostrar faixa", en = "Show track";
    TrackLock: pt_br = "Bloquear faixa", en = "Lock track";
    TrackUnlock: pt_br = "Desbloquear faixa", en = "Unlock track";
    TrackMute: pt_br = "Silenciar faixa", en = "Mute track";
    TrackUnmute: pt_br = "Reativar som da faixa", en = "Unmute track";
    TrackCollapse: pt_br = "Recolher faixa", en = "Collapse track";
    TrackExpand: pt_br = "Expandir faixa", en = "Expand track";
    QueueSubtitle: pt_br = "A edição continua responsiva enquanto os jobs renderizam em segundo plano. A fila persiste entre sessões.", en = "Editing stays responsive while jobs render in the background. The queue persists across sessions.";
    QueueTechNote: pt_br = "Nota técnica: cada job é um snapshot (bitrate/perfil/destino) tirado no momento em que entra na fila — mudanças no projeto ativo depois disso não afetam o job. Render roda em worker separado da UI (tokio::mpsc); 1 worker por padrão, configurável em Preferências.", en = "Technical note: each job is a snapshot (bitrate/profile/destination) taken the moment it enters the queue — later changes to the active project don't affect the job. Rendering runs in a worker separate from the UI (tokio::mpsc); 1 worker by default, configurable in Preferences.";
    QueueEmpty: pt_br = "Nenhum job de exportação. Use \"Adicionar exportação\" na tela do Editor.", en = "No export jobs yet. Use \"Add export\" on the Editor screen.";
    QueueMatchLoudnessLabel: pt_br = "Igualar loudness dos jobs na fila:", en = "Match loudness across queued jobs:";
    StatusRendering: pt_br = "Renderizando", en = "Rendering";
    StatusQueued: pt_br = "Na fila", en = "Queued";
    StatusPaused: pt_br = "Pausado", en = "Paused";
    StatusDone: pt_br = "Concluído", en = "Done";
    StatusFailed: pt_br = "Falhou", en = "Failed";
    CancelJob: pt_br = "Cancelar", en = "Cancel";
    RemoveJob: pt_br = "Remover", en = "Remove";
    PauseJob: pt_br = "Pausar", en = "Pause";
    Resume: pt_br = "Retomar", en = "Resume";
    OpenFolder: pt_br = "Abrir pasta", en = "Open folder";
    MoveJobUp: pt_br = "Mover para cima na fila", en = "Move up in queue";
    MoveJobDown: pt_br = "Mover para baixo na fila", en = "Move down in queue";
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
    PrefsGpuEncoder: pt_br = "Encode por GPU", en = "GPU encoding";
    GpuEncoderAuto: pt_br = "Automático", en = "Auto";
    GpuEncoderCpu: pt_br = "CPU", en = "CPU";
    GpuEncoderNvenc: pt_br = "NVIDIA (NVENC)", en = "NVIDIA (NVENC)";
    GpuEncoderQuickSync: pt_br = "Intel (Quick Sync)", en = "Intel (Quick Sync)";
    GpuEncoderAmf: pt_br = "AMD (AMF)", en = "AMD (AMF)";
    GpuEncoderVaapi: pt_br = "Linux (VAAPI Intel/AMD)", en = "Linux (VAAPI Intel/AMD)";
    GpuEncoderVideoToolbox: pt_br = "Apple (VideoToolbox)", en = "Apple (VideoToolbox)";
    PrefsPreviewQuality: pt_br = "Qualidade do preview", en = "Preview quality";
    PreviewQualityLow: pt_br = "360p", en = "360p";
    PreviewQualityMedium: pt_br = "480p", en = "480p";
    PreviewQualityHigh: pt_br = "720p", en = "720p";
    PrefsHardwareDecode: pt_br = "Aceleração de hardware no preview", en = "Hardware-accelerated preview decoding";
    PrefsHardwareDecodeHint: pt_br = "Usa VideoToolbox, NVDEC, Quick Sync ou VAAPI quando disponível e volta para CPU automaticamente.", en = "Uses VideoToolbox, NVDEC, Quick Sync, or VAAPI when available and falls back to CPU automatically.";
    Browse: pt_br = "Procurar", en = "Browse";
    PrefsProject: pt_br = "Projeto", en = "Project";
    PrefsAutosaveInterval: pt_br = "Intervalo de autosave", en = "Autosave interval";
    PrefsTelemetryEnabled: pt_br = "Telemetria local ativada (uso/erros, nunca enviado)", en = "Local telemetry enabled (usage/errors, never sent)";
    PrefsLayoutScope: pt_br = "Layout dos painéis", en = "Panel layout";
    LayoutScopePerUser: pt_br = "Por usuário", en = "Per user";
    LayoutScopePerProject: pt_br = "Por projeto", en = "Per project";
    PrefsErrorReporting: pt_br = "Relatório de erros remoto", en = "Remote error reporting";
    PrefsErrorReportingEnabled: pt_br = "Enviar relatórios de erro automaticamente (dados sanitizados, sem conteúdo pessoal)", en = "Automatically send error reports (sanitized, no personal content)";
    PrefsErrorReportingHint: pt_br = "Desativado por padrão. Nomes de arquivo, projetos, URLs e credenciais nunca são enviados — veja a pré-visualização abaixo.", en = "Disabled by default. File names, project names, URLs, and credentials are never sent — see the preview below.";
    PrefsErrorReportingQueueSize: pt_br = "Relatórios pendentes na fila local", en = "Reports pending in the local queue";
    PrefsErrorReportingDeleteQueue: pt_br = "Excluir relatórios pendentes", en = "Delete pending reports";
    PrefsShortcuts: pt_br = "Atalhos de teclado", en = "Keyboard shortcuts";
    TableAction: pt_br = "Ação", en = "Action";
    TableShortcut: pt_br = "Atalho", en = "Shortcut";
    ShortcutSplit: pt_br = "Dividir clipe (split)", en = "Split clip";
    ShortcutPlayPause: pt_br = "Play / Pause", en = "Play / Pause";
    SeekToStart: pt_br = "Ir para o início", en = "Seek to start";
    SeekToEnd: pt_br = "Ir para o fim", en = "Seek to end";
    StepFrameBack: pt_br = "Quadro anterior", en = "Previous frame";
    StepFrameForward: pt_br = "Próximo quadro", en = "Next frame";
    PreviewLoopToggle: pt_br = "Repetir", en = "Loop";
    ShortcutCopyFormatting: pt_br = "Copiar formatação", en = "Copy formatting";
    ShortcutPasteFormatting: pt_br = "Colar formatação", en = "Paste formatting";
    ShortcutAddOpacityMarker: pt_br = "Adicionar marcador de opacidade", en = "Add opacity marker";
    ShortcutUndo: pt_br = "Desfazer", en = "Undo";
    ShortcutRedo: pt_br = "Refazer", en = "Redo";
    BindingPressAnyKey: pt_br = "Pressione uma tecla...", en = "Press any key...";
    BindingChange: pt_br = "Alterar", en = "Change";
    ExportFileExistsTitle: pt_br = "Arquivo já existe", en = "File already exists";
    ExportFileExistsBody: pt_br = "Já existe um arquivo chamado \"{name}\" nesse destino.", en = "A file named \"{name}\" already exists at that destination.";
    ExportFileExistsOverwrite: pt_br = "Sobrescrever", en = "Overwrite";
    ExportFileExistsRename: pt_br = "Renomear automaticamente", en = "Rename automatically";
    ExportFileExistsRenamedTo: pt_br = "Novo nome: {name}", en = "New name: {name}";
    AutosaveFound: pt_br = "Foi encontrado um autosave mais recente para este projeto. Deseja restaurá-lo?", en = "A more recent autosave was found for this project. Do you want to restore it?";
    AutosaveRestore: pt_br = "Restaurar autosave", en = "Restore autosave";
    AutosaveDiscard: pt_br = "Descartar", en = "Discard";
    ExportPlatformPresetLabel: pt_br = "Predefinição de plataforma:", en = "Platform preset:";
    PreviewScopesToggle: pt_br = "Waveform / Vetorscópio", en = "Waveform / Vectorscope";
    SnapToggle: pt_br = "Snap", en = "Snap";
    SnapToggleHint: pt_br = "Ativa/desativa o encaixe magnético na timeline (Alt inverte temporariamente)", en = "Toggles magnetic snapping in the timeline (Alt temporarily inverts it)";
    PreviewAudioLevelMeter: pt_br = "Nível de áudio (pico / RMS)", en = "Audio level (peak / RMS)";
    PropStereoMeter: pt_br = "Medidor estéreo (L / R)", en = "Stereo meter (L / R)";
    TimelineIndexToggle: pt_br = "Marcadores", en = "Markers";
    TimelineIndexTitle: pt_br = "Índice da timeline", en = "Timeline Index";
    TimelineIndexSearchHint: pt_br = "Buscar marcadores...", en = "Search markers...";
    TimelineIndexEmpty: pt_br = "Nenhum marcador encontrado.", en = "No markers found.";
    TranscriptPanelToggle: pt_br = "Transcrição", en = "Transcript";
    TranscriptPanelTitle: pt_br = "Transcrição", en = "Transcript";
    TranscriptPanelSearchHint: pt_br = "Buscar na transcrição...", en = "Search transcript...";
    TranscriptPanelEmpty: pt_br = "Nenhuma palavra encontrada.", en = "No words found.";
    TranscriptPanelNoClip: pt_br = "Nenhum clipe de vídeo no cursor.", en = "No video clip at the playhead.";
    TranscriptPanelNoTranscript: pt_br = "Este clipe ainda não tem transcrição. Use \"Transcrever\" na tela de Mídia.", en = "This clip has no transcript yet. Use \"Transcribe\" on the Media screen.";
    TranscriptSearchProject: pt_br = "Pesquisar no projeto", en = "Search in project";
    TranscriptProjectSearchHint: pt_br = "Buscar em todo o projeto...", en = "Search whole project...";
    TranscriptProjectEmpty: pt_br = "Nenhum resultado no projeto.", en = "No results in the project.";
    DetectSpeechEdits: pt_br = "✂️ Edições de fala", en = "✂️ Speech Edits";
    TranscriptProposalsSelectClipFirst: pt_br = "Coloque o cursor sobre um clipe de vídeo para detectar edições de fala.", en = "Place the playhead on a video clip to detect speech edits.";
    TranscriptProposalsNoTranscript: pt_br = "Este clipe não tem transcrição. Transcreva-o na tela de Mídia primeiro.", en = "This clip has no transcript. Transcribe it on the Media screen first.";
    TranscriptProposalsEmpty: pt_br = "Nenhuma edição de fala detectada.", en = "No speech edits detected.";
    TranscriptProposalsTitle: pt_br = "Revisar edições de fala", en = "Review Speech Edits";
    TranscriptProposalsApply: pt_br = "Aplicar cortes selecionados", en = "Apply selected cuts";
    TranscriptProposalDeadAir: pt_br = "Silêncio longo", en = "Long pause";
    TranscriptProposalFillerWord: pt_br = "Palavra de preenchimento", en = "Filler word";
    TranscriptProposalRetake: pt_br = "Retomada", en = "Retake";
    TranscriptProposalRepeatedPhrase: pt_br = "Frase repetida", en = "Repeated phrase";
    TranscriptProposalRange: pt_br = "{start} — {end} ({duration}s)", en = "{start} — {end} ({duration}s)";
    TimelineIndexLabelHint: pt_br = "Descrição do marcador", en = "Marker description";
    RemoveMarker: pt_br = "Remover marcador", en = "Remove marker";
    TimelineIndexAddStandard: pt_br = "+ Marcador", en = "+ Marker";
    TimelineIndexAddToDo: pt_br = "+ Tarefa", en = "+ To Do";
    TimelineIndexAddChapter: pt_br = "+ Capítulo", en = "+ Chapter";
    AnalyzeMenu: pt_br = "🔍 Detectar/Analisar", en = "🔍 Detect/Analyze";
    DetectSilence: pt_br = "🔇 Detectar silêncio", en = "🔇 Detect Silence";
    SilenceReviewSelectClipFirst: pt_br = "Selecione um clipe na faixa que deseja escanear.", en = "Select a clip on the track you want to scan.";
    SilenceReviewTitle: pt_br = "Revisar silêncios detectados", en = "Review Detected Silence";
    SilenceReviewEmpty: pt_br = "Nenhum trecho de silêncio encontrado.", en = "No silent stretches found.";
    SilenceReviewGapLabel: pt_br = "{start} — {end} ({duration}s)", en = "{start} — {end} ({duration}s)";
    SilenceReviewApply: pt_br = "Aplicar cortes selecionados", en = "Apply selected cuts";
    ExportCollabBundle: pt_br = "📦 Exportar pacote de colaboração...", en = "📦 Export Collaboration Bundle...";
    ImportCollabBundle: pt_br = "📦 Importar pacote de colaboração...", en = "📦 Import Collaboration Bundle...";
    CollabBundleExported: pt_br = "Pacote de colaboração exportado.", en = "Collaboration bundle exported.";
    ExportOtio: pt_br = "Exportar OpenTimelineIO (.otio)...", en = "Export OpenTimelineIO (.otio)...";
    OtioExported: pt_br = "Arquivo .otio exportado.", en = ".otio file exported.";
    ImportOtio: pt_br = "Importar OpenTimelineIO (.otio)...", en = "Import OpenTimelineIO (.otio)...";
    ImportOtioWithMediaRoot: pt_br = "Importar OpenTimelineIO com raiz de mídia...", en = "Import OpenTimelineIO with media root...";
    OtioImported: pt_br = "Arquivo .otio importado em uma nova sequência.", en = ".otio file imported into a new sequence.";
    OtioImportedWithWarnings: pt_br = "Arquivo .otio importado em uma nova sequência; {n} item(ns) não pôde(puderam) ser trazido(s) sem aproximação.", en = ".otio file imported into a new sequence; {n} item(s) could not be brought in without approximation.";
    DetectChapters: pt_br = "🎬 Detectar capítulos", en = "🎬 Detect Chapters";
    SceneCutDetectionNone: pt_br = "Nenhum corte de cena detectado.", en = "No scene cuts detected.";
    ChapterDefaultLabel: pt_br = "Capítulo {n}", en = "Chapter {n}";
    ExportChapters: pt_br = "Exportar capítulos (.txt)", en = "Export chapters (.txt)";
    ExportChaptersNone: pt_br = "Nenhum marcador de capítulo para exportar.", en = "No chapter markers to export.";
    ExportChaptersDone: pt_br = "Capítulos exportados.", en = "Chapters exported.";
    SnapshotButton: pt_br = "Capturar quadro", en = "Capture frame";
    SnapshotNoFrame: pt_br = "Nenhum quadro para capturar ainda.", en = "No frame to capture yet.";
    SnapshotSaved: pt_br = "Captura de tela salva.", en = "Snapshot saved.";
    AddMarkerButton: pt_br = "Adicionar marcador", en = "Add marker";
    MarkerAdded: pt_br = "Marcador adicionado.", en = "Marker added.";
    PreviewZoom50: pt_br = "50%", en = "50%";
    PreviewZoomFit: pt_br = "Ajustar", en = "Fit";
    PreviewZoom100: pt_br = "100%", en = "100%";
    ProgramMonitor: pt_br = "Monitor de programa", en = "Program monitor";
    AudioRoleUnspecified: pt_br = "Papel de áudio: não definido", en = "Audio role: unspecified";
    AudioRoleGameAudio: pt_br = "Papel de áudio: áudio do jogo", en = "Audio role: game audio";
    AudioRoleMic: pt_br = "Papel de áudio: microfone", en = "Audio role: mic";
    AudioRoleMusic: pt_br = "Papel de áudio: música", en = "Audio role: music";
    DetectHighlights: pt_br = "⭐ Detectar destaques", en = "⭐ Detect Highlights";
    HighlightDetectionNeedsBothRoles: pt_br = "Marque uma faixa como áudio do jogo e outra como microfone (cabeçalho da faixa) antes de detectar destaques.", en = "Tag one track as game audio and another as mic (track header) before detecting highlights.";
    HighlightDetectionNone: pt_br = "Nenhum destaque detectado.", en = "No highlights detected.";
    HighlightDefaultLabel: pt_br = "Destaque {n}", en = "Highlight {n}";
    ImportGameplayEvents: pt_br = "🎮 Importar eventos", en = "🎮 Import Events";
    GameplayEventsImportReadFailed: pt_br = "Não foi possível ler o arquivo de eventos", en = "Could not read the events file";
    GameplayEventsImportInvalid: pt_br = "Arquivo de eventos inválido", en = "Invalid events file";
    GameplayEventsImportNoMatchingClip: pt_br = "Nenhum clipe na timeline cobre os eventos deste arquivo (a gravação não é usada, ou todos os eventos ficam fora do trecho cortado dos clipes que a usam).", en = "No clip on the timeline covers this events file's events (the recording isn't used, or every event falls outside the trimmed range of the clips that use it).";
    GameplayEventsImportNothingNew: pt_br = "Nenhum evento novo para importar (já importados anteriormente).", en = "Nothing new to import (already imported earlier).";
    GameplayEventsImportSuccess: pt_br = "{n} marcador(es) de evento importado(s).", en = "{n} event marker(s) imported.";
    PrefsGameEventAllowlists: pt_br = "Perfis de eventos por jogo", en = "Per-game event profiles";
    PrefsGameEventAllowlistsHint: pt_br = "Restringe quais tipos de evento são importados e define pré/pós-roll padrão para arquivos de eventos cujo \"game_id\" corresponda.", en = "Restricts which event kinds import and sets default pre/post-roll for event files whose \"game_id\" matches.";
    GameEventAllowlistGameIdHint: pt_br = "id do jogo", en = "game id";
    AddGameEventAllowlist: pt_br = "+ Adicionar jogo", en = "+ Add game";
    GameEventKindKill: pt_br = "Abate", en = "Kill";
    GameEventKindDeath: pt_br = "Morte", en = "Death";
    GameEventKindAssist: pt_br = "Assistência", en = "Assist";
    GameEventKindObjective: pt_br = "Objetivo", en = "Objective";
    GameEventKindBookmark: pt_br = "Marcador", en = "Bookmark";
    GameEventAllowlistPreRoll: pt_br = "pré-roll padrão", en = "default pre-roll";
    GameEventAllowlistPostRoll: pt_br = "pós-roll padrão", en = "default post-roll";
    ShortsPack: pt_br = "🎞 Pacote de shorts", en = "🎞 Shorts Pack";
    ShortsPackNoHighlights: pt_br = "Nenhum destaque detectado ainda -- use \"Detectar destaques\" primeiro.", en = "No highlights detected yet -- use \"Detect Highlights\" first.";
    ShortsPackQueued: pt_br = "{queued} shorts enfileirados ({skipped} ignorados).", en = "{queued} shorts queued ({skipped} skipped).";
    ShortsPackReframing: pt_br = "Reenquadrando {n} clipe(s) antes de montar o pack...", en = "Reframing {n} clip(s) before building the pack...";
    ExportAspectRatioLabel: pt_br = "Proporção:", en = "Aspect ratio:";
    ExportLoudnessLabel: pt_br = "Loudness:", en = "Loudness:";
    ExportLoudnessCustom: pt_br = "{value} LUFS (personalizado)", en = "{value} LUFS (custom)";
    ExportSizeEstimate: pt_br = "~{size} estimado", en = "~{size} estimated";
    HomeCtxRename: pt_br = "Configurações do projeto...", en = "Project settings...";
    SequenceTabCtxRename: pt_br = "Renomear aba...", en = "Rename tab...";
    SequenceTabCtxDuplicate: pt_br = "Duplicar aba", en = "Duplicate tab";
    SequenceTabCtxMoveLeft: pt_br = "Mover para esquerda", en = "Move left";
    SequenceTabCtxMoveRight: pt_br = "Mover para direita", en = "Move right";
    SequenceTabCtxDelete: pt_br = "Excluir aba...", en = "Delete tab...";
    TrackCtxRename: pt_br = "Renomear faixa...", en = "Rename track...";
    TrackCtxDuplicate: pt_br = "Duplicar faixa", en = "Duplicate track";
    TrackCtxMoveUp: pt_br = "Mover faixa para cima", en = "Move track up";
    TrackCtxMoveDown: pt_br = "Mover faixa para baixo", en = "Move track down";
    TrackCtxDelete: pt_br = "Excluir faixa...", en = "Delete track...";
    RenameTrackTitle: pt_br = "Renomear faixa", en = "Rename track";
    DeleteTrackTitle: pt_br = "Excluir faixa", en = "Delete track";
    DeleteTrackConfirm: pt_br = "Excluir", en = "Delete";
    SequenceTabDragHint: pt_br = "Arraste para reordenar", en = "Drag to reorder";
    RenameSequenceTitle: pt_br = "Renomear aba", en = "Rename tab";
    DeleteSequenceTitle: pt_br = "Excluir aba", en = "Delete tab";
    DeleteSequenceConfirm: pt_br = "Excluir", en = "Delete";
    DeleteSequenceCompoundClipWarning: pt_br = "⚠ Esta sequência é usada como clipe composto em: {sequences}. Excluí-la vai deixar essas referências quebradas.", en = "⚠ This sequence is used as a compound clip in: {sequences}. Deleting it will leave those references broken.";
    RenameProjectTitle: pt_br = "Configurações do projeto", en = "Project settings";
    RenameProjectConfirm: pt_br = "Salvar", en = "Save";
    ProjectNameLabel: pt_br = "Nome", en = "Name";
    ProjectSummaryLabel: pt_br = "Descrição", en = "Description";
    CrashDetected: pt_br = "oca não foi encerrado corretamente na sessão anterior. Reabra seus projetos para verificar se há autosaves de recuperação.", en = "oca did not exit cleanly in the previous session. Reopen your projects to check for recovery autosaves.";
    CrashReviewOffer: pt_br = "oca travou na última sessão. Deseja enviar um relatório de erro sanitizado para ajudar a corrigir isso?", en = "oca crashed in the last session. Would you like to send a sanitized error report to help fix this?";
    CrashReviewShowPayload: pt_br = "Ver dados que seriam enviados", en = "View data that would be sent";
    CrashReviewSendOnce: pt_br = "Enviar uma vez", en = "Send once";
    CrashReviewAlwaysSend: pt_br = "Sempre enviar relatórios de erro", en = "Always send error reports";
    CrashReviewDoNotSend: pt_br = "Não enviar", en = "Do not send";

    AddTextTrack: pt_br = "T+ Texto", en = "T+ Text";
    DefaultTextTrackName: pt_br = "Texto", en = "Text";
    AddTextClip: pt_br = "+ Adicionar texto", en = "+ Add text";
    DefaultTextContent: pt_br = "Texto", en = "Text";
    TextContentHint: pt_br = "Digite seu texto", en = "Type your text";
    SelectedTextClip: pt_br = "Sobreposição de texto", en = "Text overlay";
    NoTextClipSelected: pt_br = "Nenhuma sobreposição selecionada", en = "No overlay selected";
    PropTextContent: pt_br = "Texto", en = "Text";
    PropTextFontFamily: pt_br = "Fonte", en = "Font";
    PropTextFontStyle: pt_br = "Estilo", en = "Style";
    PropTextFontWeight: pt_br = "Peso da fonte", en = "Font weight";
    PropTextFontSize: pt_br = "Tamanho da fonte", en = "Font size";
    TextFontLato: pt_br = "Lato · Sans moderna", en = "Lato · Modern sans";
    TextFontBebasNeue: pt_br = "Bebas Neue · Título", en = "Bebas Neue · Display";
    TextFontPlayfairDisplay: pt_br = "Playfair Display · Serifada", en = "Playfair Display · Serif";
    TextFontPatrickHand: pt_br = "Patrick Hand · Manuscrita", en = "Patrick Hand · Handwritten";
    TextFontAnonymousPro: pt_br = "Anonymous Pro · Monoespaçada", en = "Anonymous Pro · Monospace";
    TextFontArchivoBlack: pt_br = "Archivo Black · Legenda bold", en = "Archivo Black · Bold caption";
    TextFontUnknown: pt_br = "Fonte desconhecida", en = "Unknown font";
    TextFontSearchHint: pt_br = "Buscar fonte...", en = "Search font...";
    FontCategorySans: pt_br = "Sem serifa", en = "Sans";
    FontCategoryDisplay: pt_br = "Destaque", en = "Display";
    FontCategorySerif: pt_br = "Serifada", en = "Serif";
    FontCategoryHandwritten: pt_br = "Manuscrita", en = "Handwritten";
    FontCategoryMonospace: pt_br = "Monoespaçada", en = "Monospace";
    FontCategoryInternational: pt_br = "Internacional", en = "International";
    TextFontRegular: pt_br = "Regular", en = "Regular";
    TextFontBold: pt_br = "Negrito", en = "Bold";
    PropTextDirection: pt_br = "Direção do texto", en = "Text direction";
    TextDirectionAuto: pt_br = "Automática", en = "Auto";
    TextDirectionLtr: pt_br = "Esquerda para direita", en = "Left to right";
    TextDirectionRtl: pt_br = "Direita para esquerda", en = "Right to left";
    PropTextAlign: pt_br = "Alinhamento", en = "Alignment";
    TextAlignAuto: pt_br = "Automático", en = "Auto";
    TextAlignLeft: pt_br = "Esquerda", en = "Left";
    TextAlignCenter: pt_br = "Centro", en = "Center";
    TextAlignRight: pt_br = "Direita", en = "Right";
    TextAlignStart: pt_br = "Início", en = "Start";
    TextAlignEnd: pt_br = "Fim", en = "End";
    TextBidiControlWarning: pt_br = "⚠ Este texto contém caracteres invisíveis de controle de direção que podem alterar como ele é exibido.", en = "⚠ This text contains invisible directional-control characters that can change how it renders.";
    PropTextColor: pt_br = "Cor do texto", en = "Text color";
    PropTextBackgroundEnabled: pt_br = "Fundo", en = "Background";
    PropTextBackgroundColor: pt_br = "Cor do fundo", en = "Background color";
    PropTextBackgroundPadding: pt_br = "Espaçamento", en = "Padding";
    PropTextBackgroundRadius: pt_br = "Cantos arredondados", en = "Corner radius";
    PropTextHighlightEnabled: pt_br = "Destacar palavra falada", en = "Highlight spoken word";
    PropTextHighlightColor: pt_br = "Cor do destaque", en = "Highlight color";
    TextColorPickerTitle: pt_br = "Escolher cor", en = "Choose color";
    TextColorPickerPresets: pt_br = "Cores predefinidas", en = "Preset colors";
    TextColorPickerManual: pt_br = "Valor manual", en = "Manual value";
    TextColorPickerManualHint: pt_br = "#RRGGBB, #RRGGBBAA, rgb(...) ou rgba(...)", en = "#RRGGBB, #RRGGBBAA, rgb(...), or rgba(...)";
    TextColorPickerUseValue: pt_br = "Usar valor", en = "Use value";
    TextColorPickerInvalid: pt_br = "Cor inválida. Use HEX, RGB ou RGBA.", en = "Invalid color. Use HEX, RGB, or RGBA.";
    TextColorPickerApply: pt_br = "Aplicar", en = "Apply";
    PropTextPosX: pt_br = "Posição X", en = "Position X";
    PropTextPosY: pt_br = "Posição Y", en = "Position Y";
    PropTextPosXKeyframes: pt_br = "Posição X (keyframes)", en = "Position X (keyframes)";
    PropTextPosYKeyframes: pt_br = "Posição Y (keyframes)", en = "Position Y (keyframes)";
    TextPositionKeyframesExportNote: pt_br = "quando definido, substitui a posição constante desse eixo na exportação (movimento animado) — sem efeito no preview ao vivo.", en = "when set, overrides this axis's constant position on export (animated movement) — no live preview effect.";
    PropTextScaleKeyframes: pt_br = "Escala (keyframes)", en = "Scale (keyframes)";
    TextScaleKeyframesExportNote: pt_br = "anima o tamanho do texto ao redor da sua própria posição durante a exportação — sem efeito no preview ao vivo.", en = "animates the text's size around its own position on export — no live preview effect.";
    PropTextRotationKeyframes: pt_br = "Rotação (keyframes)", en = "Rotation (keyframes)";
    TextRotationKeyframesExportNote: pt_br = "anima a rotação do texto ao redor da sua própria posição durante a exportação — sem efeito no preview ao vivo.", en = "animates the text's rotation around its own position on export — no live preview effect.";
    PropTextDuration: pt_br = "Duração (s)", en = "Duration (s)";
    PropTextStart: pt_br = "Início (s)", en = "Start (s)";
    TextExportNote: pt_br = "fontes embutidas e fundo usam o mesmo rasterizador no preview e no export.", en = "bundled fonts and backgrounds use the same rasterizer in preview and export.";

    AddShapeTrack: pt_br = "S+ Forma", en = "S+ Shape";
    DefaultShapeTrackName: pt_br = "Forma", en = "Shape";
    AddShapeClip: pt_br = "+ Adicionar forma", en = "+ Add shape";
    DrawCustomShape: pt_br = "✎ Desenhar forma", en = "✎ Draw shape";
    ShapeDrawHint: pt_br = "Clique no preview para adicionar pontos (mín. 3) · Enter conclui · Esc cancela", en = "Click the preview to add points (min. 3) · Enter finishes · Esc cancels";
    ShapeDrawNeedsPreview: pt_br = "Carregue um clipe no preview antes de desenhar uma forma", en = "Load a clip in the preview before drawing a shape";
    SelectedShapeClip: pt_br = "Forma geométrica", en = "Geometric shape";
    NoShapeClipSelected: pt_br = "Nenhuma forma selecionada", en = "No shape selected";
    PropShapeKind: pt_br = "Tipo", en = "Kind";
    ShapePresetEllipse: pt_br = "Elipse/círculo", en = "Ellipse/circle";
    ShapePresetRectangle: pt_br = "Retângulo/quadrado", en = "Rectangle/square";
    ShapePresetTriangle: pt_br = "Triângulo", en = "Triangle";
    ShapePresetTrapezoid: pt_br = "Trapézio", en = "Trapezoid";
    ShapePresetArrow: pt_br = "Seta", en = "Arrow";
    ShapePresetCustom: pt_br = "Personalizado", en = "Custom";
    PropShapeColor: pt_br = "Cor", en = "Color";
    PropShapePosX: pt_br = "Posição X", en = "Position X";
    PropShapePosY: pt_br = "Posição Y", en = "Position Y";
    PropShapePosXKeyframes: pt_br = "Posição X (keyframes)", en = "Position X (keyframes)";
    PropShapePosYKeyframes: pt_br = "Posição Y (keyframes)", en = "Position Y (keyframes)";
    ShapePositionKeyframesExportNote: pt_br = "quando definido, substitui a posição constante desse eixo na exportação (recorte/panorâmica animados) — sem efeito no preview ao vivo.", en = "when set, overrides this axis's constant position on export (animated pan/reveal) — no live preview effect.";
    PropShapeWidth: pt_br = "Largura", en = "Width";
    PropShapeHeight: pt_br = "Altura", en = "Height";
    PropShapeWidthKeyframes: pt_br = "Largura (keyframes)", en = "Width (keyframes)";
    PropShapeHeightKeyframes: pt_br = "Altura (keyframes)", en = "Height (keyframes)";
    ShapeSizeKeyframesExportNote: pt_br = "quando definido, substitui o tamanho constante desse eixo na exportação (crescimento/redução animados) — sem efeito no preview ao vivo.", en = "when set, overrides this axis's constant size on export (animated grow/shrink) — no live preview effect.";
    PropShapeRotation: pt_br = "Rotação", en = "Rotation";
    PropShapeRotationKeyframes: pt_br = "Rotação (keyframes)", en = "Rotation (keyframes)";
    ShapeRotationKeyframesExportNote: pt_br = "quando definido, substitui a rotação constante na exportação (giro animado) — sem efeito no preview ao vivo.", en = "when set, overrides the constant rotation on export (animated spin) — no live preview effect.";
    PropTextOpacityKeyframes: pt_br = "Opacidade (keyframes)", en = "Opacity (keyframes)";
    TextOpacityKeyframesExportNote: pt_br = "cria um fade de entrada/saída no texto durante a exportação — sem efeito no preview ao vivo.", en = "creates a fade in/out for the text on export — no live preview effect.";
    PropShapeStroke: pt_br = "Espessura do contorno (px)", en = "Outline thickness (px)";
    PropShapeStrokeHint: pt_br = "0 = preenchido", en = "0 = filled";
    PropShapeDuration: pt_br = "Duração (s)", en = "Duration (s)";
    PropShapeStart: pt_br = "Início (s)", en = "Start (s)";
    ShapeExportNote: pt_br = "renderizado via geq no export — preview não suportado ainda.", en = "rendered via geq on export — preview not supported yet.";
    PropShapeVertices: pt_br = "Vértices", en = "Vertices";
    ShapeVerticesHint: pt_br = "arraste os valores X/Y para desenhar uma forma personalizada — funciona em qualquer predefinição, já que todas são polígonos por baixo dos panos (exceto elipse/círculo).", en = "drag the X/Y values to draw a custom shape — works on any preset, since they're all polygons underneath (except ellipse/circle).";
    ShapeAddVertex: pt_br = "+ Adicionar vértice", en = "+ Add vertex";
    CreateMulticamGroup: pt_br = "🎬 Sincronizar multicam", en = "🎬 Sync Multicam";
    MulticamGroupNeedsTwoVideoTracks: pt_br = "Adicione pelo menos duas faixas de vídeo (os ângulos) antes de sincronizar um grupo multicam.", en = "Add at least two video tracks (the angles) before syncing a multicam group.";
    MulticamGroupNeedsAudio: pt_br = "Cada ângulo precisa de uma faixa de áudio própria para a sincronização automática por forma de onda.", en = "Every angle needs its own audio track for automatic waveform-based sync.";
    MulticamGroupSynced: pt_br = "Grupo multicam sincronizado com {n} ângulos -- use as teclas 1-9 no playhead para trocar de ângulo.", en = "Multicam group synced with {n} angles -- use keys 1-9 at the playhead to switch angles.";
    MulticamNoActiveGroup: pt_br = "Nenhum grupo multicam ainda -- clique em \"Sincronizar multicam\" primeiro.", en = "No multicam group yet -- click \"Sync Multicam\" first.";
    MulticamSwitchFailed: pt_br = "Não foi possível trocar para o ângulo {n} nesta posição -- talvez essa fonte não tenha imagem aqui.", en = "Couldn't switch to angle {n} at this position -- that source may have no footage here.";
    SmartBinAll: pt_br = "Tudo", en = "All";
    SmartBinNew: pt_br = "+ Nova pasta", en = "+ New Bin";
    MediaFilterFavorites: pt_br = "★ Favoritos", en = "★ Favorites";
    MediaFilterRecent: pt_br = "Recentes", en = "Recent";
    ToggleFavorite: pt_br = "Marcar/desmarcar como favorito", en = "Toggle favorite";
    SmartBinEditTitle: pt_br = "Pasta inteligente", en = "Smart bin";
    SmartBinNameLabel: pt_br = "Nome", en = "Name";
    SmartBinKindLabel: pt_br = "Tipo", en = "Type";
    SmartBinKindAny: pt_br = "Qualquer", en = "Any";
    SmartBinKindVideo: pt_br = "Vídeo", en = "Video";
    SmartBinKindAudio: pt_br = "Áudio", en = "Audio";
    SmartBinNameContainsLabel: pt_br = "Nome do arquivo contém", en = "File name contains";
    SmartBinAudioLabel: pt_br = "Tem áudio", en = "Has audio";
    SmartBinAudioAny: pt_br = "Tanto faz", en = "Either";
    SmartBinAudioYes: pt_br = "Sim", en = "Yes";
    SmartBinAudioNo: pt_br = "Não", en = "No";
    SmartBinSave: pt_br = "Salvar", en = "Save";
    SmartBinDelete: pt_br = "Excluir", en = "Delete";
}

/// The breadcrumb's title for `screen` (e.g. `Screen::Queue` -> "Fila de exportação" — longer
/// than its [`nav_label`], which is space-constrained in the rail).
pub fn screen_title(locale: Locale, screen: Screen) -> &'static str {
    match screen {
        Screen::Home => Text::ScreenTitleHome.tr(locale),
        Screen::Editor => Text::ScreenTitleEditor.tr(locale),
        Screen::Library => Text::ScreenTitleLibrary.tr(locale),
        Screen::SoundLibrary => Text::ScreenTitleSoundLibrary.tr(locale),
        Screen::Queue => Text::ScreenTitleQueue.tr(locale),
        Screen::WatchFolder => Text::WatchFolderTitle.tr(locale),
    }
}

/// The nav rail's short label for `screen` (e.g. `Screen::Queue` -> "Fila").
pub fn nav_label(locale: Locale, screen: Screen) -> &'static str {
    match screen {
        Screen::Home => Text::NavHome.tr(locale),
        Screen::Editor => Text::NavEditor.tr(locale),
        Screen::Library => Text::NavLibrary.tr(locale),
        Screen::SoundLibrary => Text::NavSoundLibrary.tr(locale),
        Screen::Queue => Text::NavQueue.tr(locale),
        Screen::WatchFolder => Text::NavWatchFolder.tr(locale),
    }
}

/// Formats the media library panel's item count (e.g. "18 itens"/"18 items"), matching
/// `oca-editor-mock.html`'s `.panel-head` secondary count text.
pub fn media_item_count_label(locale: Locale, count: usize) -> String {
    match locale {
        Locale::PtBr if count == 1 => "1 item".to_string(),
        Locale::PtBr => format!("{count} itens"),
        Locale::En if count == 1 => "1 item".to_string(),
        Locale::En => format!("{count} items"),
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

/// Default name for the `n`-th sequence tab a project's "+" button adds (e.g. "Sequência 2") —
/// the *first* sequence a project starts with instead gets [`Text::DefaultSequenceName`], a
/// distinct, non-numbered string.
pub fn sequence_name(locale: Locale, n: usize) -> String {
    match locale {
        Locale::PtBr => format!("Sequência {n}"),
        Locale::En => format!("Sequence {n}"),
    }
}

/// Localized name assigned to a sequence duplicated from `source_name`.
pub fn sequence_copy_name(locale: Locale, source_name: &str) -> String {
    match locale {
        Locale::PtBr => format!("Cópia de {source_name}"),
        Locale::En => format!("{source_name} copy"),
    }
}

/// Confirmation text for deleting a sequence. Kept here rather than in the modal so every
/// user-visible sentence remains centralized with the rest of the locale catalog.
pub fn delete_sequence_prompt(locale: Locale, name: &str) -> String {
    match locale {
        Locale::PtBr => format!(
            "Excluir a aba \"{name}\"? A timeline e as configurações desta sequência serão removidas."
        ),
        Locale::En => format!(
            "Delete the \"{name}\" tab? This sequence's timeline and settings will be removed."
        ),
    }
}

/// Section 49's own "Deleting a track containing clips requires confirmation" — the prompt text
/// for `App::deleting_track`'s modal.
pub fn delete_track_prompt(locale: Locale, name: &str) -> String {
    match locale {
        Locale::PtBr => {
            format!("Excluir a faixa \"{name}\"? Todos os clipes nela contidos serão removidos.")
        }
        Locale::En => {
            format!("Delete the \"{name}\" track? Every clip on it will be removed.")
        }
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

/// The export queue's secondary line for `job` —
/// `"-14 LUFS · 42 Mbps · ~342 MB · /export/…"` for a normal job with a non-zero duration,
/// or `"Erro: <message>"`/`"Error: <message>"` for a failed one. The size estimate is omitted
/// when the job's segments carry no duration (e.g. in tests with an empty segment list).
pub fn job_detail_line(locale: Locale, job: &ExportJob) -> String {
    match &job.status {
        ExportJobStatus::Failed { message } => {
            format!("{}: {message}", Text::ErrorPrefix.tr(locale))
        }
        _ => {
            let total_secs: f64 = job
                .segments
                .iter()
                .map(|s| {
                    let speed = if s.speed_factor > 0.0 {
                        s.speed_factor as f64
                    } else {
                        1.0
                    };
                    (s.source_out_secs - s.source_in_secs) / speed
                })
                .sum();
            let size_str = if total_secs > 0.0 {
                let mb = (job.canvas.bit_rate_bps as f64 * total_secs) / 8.0 / 1_048_576.0;
                if mb >= 1024.0 {
                    format!(" · ~{:.1} GB", mb / 1024.0)
                } else {
                    format!(" · ~{:.0} MB", mb)
                }
            } else {
                String::new()
            };
            format!(
                "-{:.0} LUFS · {:.0} Mbps{} · {}",
                -job.target_lufs,
                job.canvas.bit_rate_bps as f32 / 1_000_000.0,
                size_str,
                job.output_path
            )
        }
    }
}

#[cfg(test)]
#[path = "i18n/i18n_test.rs"]
mod tests;
