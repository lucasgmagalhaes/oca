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
    NavSoundLibrary: pt_br = "Música/SFX", en = "Music/SFX";
    NavQueue: pt_br = "Fila", en = "Queue";
    NavPrefs: pt_br = "Ajustes", en = "Settings";

    ScreenTitleHome: pt_br = "Início", en = "Home";
    ScreenTitleEditor: pt_br = "Editor", en = "Editor";
    ScreenTitleLibrary: pt_br = "Mídia", en = "Media";
    ScreenTitleSoundLibrary: pt_br = "Música e efeitos sonoros", en = "Music & sound effects";
    ScreenTitleQueue: pt_br = "Fila de exportação", en = "Export queue";
    UnsavedChanges: pt_br = "Alterações não salvas", en = "Unsaved changes";

    HomeTitle: pt_br = "Projetos recentes", en = "Recent projects";
    HomeSubtitle: pt_br = "Continue de onde parou ou comece um projeto novo.", en = "Continue where you left off or start a new project.";
    NewProject: pt_br = "＋ Novo projeto", en = "＋ New project";
    OpenProject: pt_br = "Abrir projeto…", en = "Open project…";
    UntitledProject: pt_br = "Projeto sem título", en = "Untitled project";
    HomeCtxRemove: pt_br = "Remover da lista", en = "Remove from list";
    HomeCtxShowInFinder: pt_br = "Mostrar no Finder", en = "Show in Finder";
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
    PropGain: pt_br = "Ganho do bloco", en = "Block gain";
    GainExportNote: pt_br = "afeta só a waveform por enquanto — export ainda não mixa a timeline.", en = "affects only the waveform for now — export doesn't mix the timeline yet.";
    PropFreeze: pt_br = "❄ Congelar quadro", en = "❄ Freeze frame";
    FreezeExportNote: pt_br = "afeta só a timeline por enquanto — preview e export ainda tocam a fonte normalmente.", en = "affects only the timeline for now — preview and export still play the source normally.";
    PropDeflicker: pt_br = "✨ Remover flicker", en = "✨ Remove flicker";
    DeflickerExportNote: pt_br = "aplicado no export — preview ainda não mostra.", en = "applied on export — preview doesn't show it yet.";
    PropSpeed: pt_br = "Velocidade", en = "Speed";
    SpeedExportNote: pt_br = "mostra só um selo no bloco por enquanto — ainda não reamostra áudio nem muda a duração na timeline.", en = "only shows a badge on the block for now — doesn't resample audio or change the timeline duration yet.";
    PropCrop: pt_br = "Recorte (x, y, largura, altura)", en = "Crop (x, y, width, height)";
    CropReset: pt_br = "Redefinir recorte", en = "Reset crop";
    CropExportNote: pt_br = "mostra só um selo no bloco por enquanto — preview e export ainda mostram o quadro inteiro.", en = "only shows a badge on the block for now — preview and export still show the full frame.";
    PropMask: pt_br = "Máscara", en = "Mask";
    MaskNone: pt_br = "Nenhuma", en = "None";
    MaskCircle: pt_br = "Círculo", en = "Circle";
    MaskRoundedRect: pt_br = "Retângulo arredondado", en = "Rounded rectangle";
    MaskCornerRadius: pt_br = "Raio do canto", en = "Corner radius";
    MaskExportNote: pt_br = "mostra só um selo no bloco por enquanto — preview e export ainda mostram o quadro inteiro.", en = "only shows a badge on the block for now — preview and export still show the full frame.";
    PropFlip: pt_br = "⇄ Espelhar horizontal", en = "⇄ Flip horizontal";
    FlipExportNote: pt_br = "mostra só um selo no bloco por enquanto — preview e export ainda mostram o quadro sem espelhar.", en = "only shows a badge on the block for now — preview and export still show the frame unmirrored.";
    PropColorFilter: pt_br = "Filtro de cor", en = "Color filter";
    ColorFilterNone: pt_br = "Nenhum", en = "None";
    ColorFilterBlackAndWhite: pt_br = "Preto e branco", en = "Black and white";
    ColorFilterSepia: pt_br = "Sépia", en = "Sepia";
    ColorFilterExportNote: pt_br = "mostra só uma prévia tintada no bloco por enquanto — preview e export ainda mostram as cores originais.", en = "only shows a tinted preview on the block for now — preview and export still show the original colors.";
    PropLut: pt_br = "LUT 3D", en = "3D LUT";
    LutExportNote: pt_br = "aplicado na exportação via lut3d — sem suporte no preview ao vivo (nenhum elemento de LUT disponível na instalação do GStreamer).", en = "applied on export via lut3d — no live preview support (no LUT element available in the GStreamer install).";
    ClearLut: pt_br = "Remover", en = "Clear";
    PropLayerSize: pt_br = "Tamanho da camada", en = "Layer size";
    LayerSizeExportNote: pt_br = "redimensiona a camada no canvas — só tem efeito em faixas de overlay.", en = "resizes the layer on the canvas — only has an effect on overlay tracks.";
    PropLayerWidth: pt_br = "Largura", en = "Width";
    PropLayerHeight: pt_br = "Altura", en = "Height";
    PropVignette: pt_br = "Vinheta", en = "Vignette";
    VignetteExportNote: pt_br = "mostra só uma borda escurecida no bloco por enquanto — preview e export ainda não aplicam a vinheta.", en = "only shows a darkened border on the block for now — preview and export don't apply the vignette yet.";
    PropColorAdjust: pt_br = "Cor", en = "Color";
    PropBrightness: pt_br = "Brilho", en = "Brightness";
    PropContrast: pt_br = "Contraste", en = "Contrast";
    PropSaturation: pt_br = "Saturação", en = "Saturation";
    ColorAdjustExportNote: pt_br = "ainda não tem efeito visível em lugar nenhum — preview e export ignoram esses valores por enquanto.", en = "has no visible effect anywhere yet — preview and export ignore these values for now.";
    PropSharpen: pt_br = "Nitidez", en = "Sharpen";
    SharpenExportNote: pt_br = "ainda não tem efeito visível em lugar nenhum — preview e export ignoram esse valor por enquanto.", en = "has no visible effect anywhere yet — preview and export ignore this value for now.";
    PropChromaKey: pt_br = "🟩 Chroma key", en = "🟩 Chroma key";
    ChromaKeyColor: pt_br = "Cor:", en = "Color:";
    ChromaKeyTolerance: pt_br = "Tolerância", en = "Tolerance";
    ChromaKeyExportNote: pt_br = "mostra só um selo no bloco por enquanto — preview e export ainda não removem o fundo.", en = "only shows a badge on the block for now — preview and export don't remove the background yet.";
    PropBackgroundRemoval: pt_br = "🤖 Remoção de fundo (IA)", en = "🤖 Background removal (AI)";
    BackgroundRemovalExportNote: pt_br = "detecção por IA já roda de verdade (MODNet via ONNX), mas ainda não gera a máscara nem afeta preview ou exportação — só marca o bloco por enquanto.", en = "the AI detection itself already runs for real (MODNet via ONNX), but doesn't generate the matte or affect preview/export yet — only marks the block for now.";
    DownloadBackgroundRemovalModel: pt_br = "Baixar modelo de remoção de fundo", en = "Download background removal model";
    PrefsBackgroundRemovalModelPath: pt_br = "Caminho do modelo de remoção de fundo", en = "Background removal model path";
    TtsButton: pt_br = "🔊 Texto-pra-fala", en = "🔊 Text-to-speech";
    TtsModalTitle: pt_br = "Texto-pra-fala", en = "Text-to-speech";
    TtsGenerate: pt_br = "Gerar", en = "Generate";
    TtsNoModelConfigured: pt_br = "Nenhum modelo de voz configurado — baixe um em Ajustes primeiro.", en = "No voice model configured — download one in Preferences first.";
    TtsGenerationFailed: pt_br = "Falha ao gerar narração", en = "Narration generation failed";
    TtsGenerating: pt_br = "Gerando narração…", en = "Generating narration…";
    DownloadTtsVoice: pt_br = "Baixar voz de texto-pra-fala", en = "Download text-to-speech voice";
    PrefsTtsModelPath: pt_br = "Caminho do modelo de texto-pra-fala", en = "Text-to-speech model path";
    PropOtherEffects: pt_br = "Outros efeitos", en = "Other effects";
    PropBlur: pt_br = "Blur", en = "Blur";
    PropShake: pt_br = "Tremido", en = "Shake";
    PropGlitch: pt_br = "Glitch", en = "Glitch";
    PropPixelize: pt_br = "Pixelizar", en = "Pixelize";
    OtherEffectsExportNote: pt_br = "ainda não têm efeito visível em lugar nenhum — preview e export ignoram esses valores por enquanto.", en = "have no visible effect anywhere yet — preview and export ignore these values for now.";
    PropStabilization: pt_br = "🎥 Estabilização", en = "🎥 Stabilization";
    StabilizationExportNote: pt_br = "aplicado na exportação via deshake — sem suporte no preview ao vivo (nenhum elemento de estabilização disponível na instalação do GStreamer).", en = "applied on export via deshake — no live preview support (no stabilization element available in the GStreamer install).";
    PropTransition: pt_br = "Transição", en = "Transition";
    TransitionNone: pt_br = "Nenhuma", en = "None";
    TransitionFade: pt_br = "Fade", en = "Fade";
    TransitionHardCut: pt_br = "Corte seco", en = "Hard cut";
    TransitionSlide: pt_br = "Slide", en = "Slide";
    TransitionZoom: pt_br = "Zoom", en = "Zoom";
    PropTransitionDuration: pt_br = "Duração da transição", en = "Transition duration";
    TransitionExportNote: pt_br = "só afeta o painel de propriedades por enquanto, sem efeito no preview ou na exportação — e modela só a transição de entrada deste bloco, não uma mistura real entre dois clipes.", en = "only affects the properties panel for now, with no effect on preview or export — and models only this block's incoming transition, not a real cross-blend between two clips.";
    PropPositionKeyframes: pt_br = "Posição (keyframes)", en = "Position (keyframes)";
    PropScaleKeyframes: pt_br = "Escala (keyframes / punch-in)", en = "Scale (keyframes / punch-in)";
    PropRotationKeyframes: pt_br = "Rotação (keyframes)", en = "Rotation (keyframes)";
    PropOpacityKeyframes: pt_br = "Opacidade (keyframes)", en = "Opacity (keyframes)";
    AddKeyframe: pt_br = "+ Adicionar keyframe", en = "+ Add keyframe";
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
    OnExport: pt_br = "Ao exportar", en = "On export";
    NormalizeTo: pt_br = "Normalizar", en = "Normalize";
    BitrateFromSource: pt_br = "Bitrate = fonte", en = "Bitrate = source";
    ExportAutoNote: pt_br = "loudnorm 2-pass + true peak limiter, aplicado automático no export — sem ajuste manual por clipe.", en = "loudnorm 2-pass + true peak limiter, applied automatically on export — no per-clip manual adjustment.";
    Timeline: pt_br = "🔍 Timeline", en = "🔍 Timeline";
    TimelineEmpty: pt_br = "Este projeto ainda não tem clipes na timeline.", en = "This project doesn't have any clips on the timeline yet.";
    ContextMenuSplit: pt_br = "✂ Dividir no playhead", en = "✂ Split at playhead";
    ContextMenuDelete: pt_br = "🗑 Excluir", en = "🗑 Delete";
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
    TrackKindVideo: pt_br = "Vídeo", en = "Video";
    TrackKindAudio: pt_br = "Áudio", en = "Audio";
    TrackKindText: pt_br = "Texto", en = "Text";
    TrackKindShape: pt_br = "Forma", en = "Shape";
    PreviewUnavailable: pt_br = "Pré-visualização indisponível", en = "Preview unavailable";

    LibraryTitle: pt_br = "Biblioteca de mídia", en = "Media library";
    ImportFiles: pt_br = "⭱ Importar arquivos", en = "⭱ Import files";
    Importing: pt_br = "Importando arquivos…", en = "Importing files…";
    LibraryEmpty: pt_br = "Nenhum arquivo importado neste projeto ainda.", en = "No files imported into this project yet.";
    ProxyReady: pt_br = "Proxy 540p", en = "540p proxy";
    TranscribeAction: pt_br = "Transcrever", en = "Transcribe";
    TranscribeInProgress: pt_br = "Transcrevendo...", en = "Transcribing...";
    TranscribeNoModelConfigured: pt_br = "Configure o modelo Whisper em Ajustes antes de transcrever.", en = "Set up the Whisper model in Preferences before transcribing.";
    TranscribeNoSpeechFound: pt_br = "Nenhuma fala reconhecida no áudio.", en = "No speech recognized in the audio.";
    PrefsWhisperModelPath: pt_br = "Modelo Whisper (legendas automáticas)", en = "Whisper model (automatic subtitles)";
    PrefsSoundLibraryPath: pt_br = "Pasta da biblioteca de música/SFX", en = "Music/SFX library folder";
    PrefsReframeModelPath: pt_br = "Modelo de reenquadramento automático", en = "Auto-reframe model";
    DownloadReframeModel: pt_br = "Baixar modelo (≈2 MB)", en = "Download model (≈2 MB)";

    AutoReframeAction: pt_br = "Reenquadramento automático", en = "Auto-reframe";
    AutoReframeInProgress: pt_br = "Reenquadrando...", en = "Reframing...";
    AutoReframeNoModelConfigured: pt_br = "Configure o modelo de reenquadramento em Ajustes antes de usar.", en = "Set up the auto-reframe model in Preferences before using this.";
    AutoReframeNoSubjectFound: pt_br = "Nenhum rosto detectado — recorte centralizado aplicado.", en = "No face detected — applied a centered crop instead.";

    MotionTrackAction: pt_br = "Rastrear movimento", en = "Track motion";
    MotionTrackInProgress: pt_br = "Rastreando...", en = "Tracking...";
    MotionTrackNoFramesDecoded: pt_br = "Não foi possível decodificar quadros suficientes para rastrear.", en = "Couldn't decode enough frames to track.";

    SoundLibraryTitle: pt_br = "Música e efeitos sonoros", en = "Music & sound effects";
    SoundLibraryRescan: pt_br = "⟳ Atualizar", en = "⟳ Rescan";
    SoundLibraryMusic: pt_br = "Música", en = "Music";
    SoundLibrarySfx: pt_br = "Efeitos sonoros", en = "Sound effects";
    SoundLibraryAddToTimeline: pt_br = "+ Adicionar à timeline", en = "+ Add to timeline";
    SoundLibraryNoFolderConfigured: pt_br = "Nenhuma pasta configurada. Escolha uma pasta com subpastas \"music\"/\"sfx\" em Ajustes.", en = "No folder configured. Pick a folder with \"music\"/\"sfx\" subfolders in Preferences.";
    SoundLibraryEmpty: pt_br = "Nenhuma faixa encontrada. Adicione arquivos de áudio nas subpastas \"music\"/\"sfx\" da pasta configurada.", en = "No tracks found. Add audio files to the configured folder's \"music\"/\"sfx\" subfolders.";

    QueueTitle: pt_br = "Fila de exportação", en = "Export queue";
    AddExport: pt_br = "＋ Adicionar exportação", en = "＋ Add export";
    AddExportNeedsClip: pt_br = "Adicione ao menos um clipe de vídeo na sequência ativa primeiro", en = "Add at least one video clip to the active sequence first";
    AddVideoTrack: pt_br = "＋ Adicionar faixa de vídeo", en = "＋ Add video track";
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
    PrefsGpuEncoder: pt_br = "Encode por GPU", en = "GPU encoding";
    GpuEncoderAuto: pt_br = "Automático", en = "Auto";
    GpuEncoderCpu: pt_br = "CPU", en = "CPU";
    GpuEncoderNvenc: pt_br = "NVIDIA (NVENC)", en = "NVIDIA (NVENC)";
    GpuEncoderQuickSync: pt_br = "Intel (Quick Sync)", en = "Intel (Quick Sync)";
    GpuEncoderAmf: pt_br = "AMD (AMF)", en = "AMD (AMF)";
    Browse: pt_br = "Procurar", en = "Browse";
    PrefsProject: pt_br = "Projeto", en = "Project";
    PrefsAutosaveInterval: pt_br = "Intervalo de autosave", en = "Autosave interval";
    PrefsShortcuts: pt_br = "Atalhos de teclado", en = "Keyboard shortcuts";
    TableAction: pt_br = "Ação", en = "Action";
    TableShortcut: pt_br = "Atalho", en = "Shortcut";
    ShortcutSplit: pt_br = "Dividir clipe (split)", en = "Split clip";
    ShortcutPlayPause: pt_br = "Play / Pause", en = "Play / Pause";
    ShortcutCopyFormatting: pt_br = "Copiar formatação", en = "Copy formatting";
    ShortcutPasteFormatting: pt_br = "Colar formatação", en = "Paste formatting";
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
    ExportAspectRatioLabel: pt_br = "Proporção:", en = "Aspect ratio:";
    ExportSizeEstimate: pt_br = "~{size} estimado", en = "~{size} estimated";
    HomeCtxRename: pt_br = "Configurações do projeto...", en = "Project settings...";
    SequenceTabCtxRename: pt_br = "Renomear aba...", en = "Rename tab...";
    RenameSequenceTitle: pt_br = "Renomear aba", en = "Rename tab";
    RenameProjectTitle: pt_br = "Configurações do projeto", en = "Project settings";
    RenameProjectConfirm: pt_br = "Salvar", en = "Save";
    ProjectNameLabel: pt_br = "Nome", en = "Name";
    ProjectSummaryLabel: pt_br = "Descrição", en = "Description";
    CrashDetected: pt_br = "oca não foi encerrado corretamente na sessão anterior. Reabra seus projetos para verificar se há autosaves de recuperação.", en = "oca did not exit cleanly in the previous session. Reopen your projects to check for recovery autosaves.";

    AddTextTrack: pt_br = "T+ Texto", en = "T+ Text";
    DefaultTextTrackName: pt_br = "Texto", en = "Text";
    AddTextClip: pt_br = "+ Adicionar texto", en = "+ Add text";
    SelectedTextClip: pt_br = "Sobreposição de texto", en = "Text overlay";
    NoTextClipSelected: pt_br = "Nenhuma sobreposição selecionada", en = "No overlay selected";
    PropTextContent: pt_br = "Texto", en = "Text";
    PropTextFontSize: pt_br = "Tamanho da fonte", en = "Font size";
    PropTextColor: pt_br = "Cor do texto", en = "Text color";
    PropTextHighlightEnabled: pt_br = "Destacar palavra falada", en = "Highlight spoken word";
    PropTextPosX: pt_br = "Posição X", en = "Position X";
    PropTextPosY: pt_br = "Posição Y", en = "Position Y";
    PropTextDuration: pt_br = "Duração (s)", en = "Duration (s)";
    PropTextStart: pt_br = "Início (s)", en = "Start (s)";
    TextExportNote: pt_br = "renderizado via drawtext no export — preview não suportado ainda.", en = "rendered via drawtext on export — preview not supported yet.";

    AddShapeTrack: pt_br = "S+ Forma", en = "S+ Shape";
    DefaultShapeTrackName: pt_br = "Forma", en = "Shape";
    AddShapeClip: pt_br = "+ Adicionar forma", en = "+ Add shape";
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
    PropShapeWidth: pt_br = "Largura", en = "Width";
    PropShapeHeight: pt_br = "Altura", en = "Height";
    PropShapeRotation: pt_br = "Rotação", en = "Rotation";
    PropShapeStroke: pt_br = "Espessura do contorno (px)", en = "Outline thickness (px)";
    PropShapeStrokeHint: pt_br = "0 = preenchido", en = "0 = filled";
    PropShapeDuration: pt_br = "Duração (s)", en = "Duration (s)";
    PropShapeStart: pt_br = "Início (s)", en = "Start (s)";
    ShapeExportNote: pt_br = "renderizado via geq no export — preview não suportado ainda.", en = "rendered via geq on export — preview not supported yet.";
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
