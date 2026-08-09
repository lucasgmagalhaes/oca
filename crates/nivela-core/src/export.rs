use serde::{Deserialize, Serialize};

/// A queued render's lifecycle. `Rendering`/`Paused` carry a snapshot progress percentage;
/// the queue itself (Fase 4) will drive these via the background worker channel.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExportJobStatus {
    Queued,
    Rendering { percent: u8 },
    Paused { percent: u8 },
    Done,
    Failed { message: String },
}

/// One export job. Configuration fields are a snapshot taken when the job entered the
/// queue — later edits to the source project must not retroactively change a queued job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportJob {
    pub id: u64,
    pub title: String,
    pub target_lufs: f32,
    pub bitrate_mbps: f32,
    pub output_path: String,
    pub status: ExportJobStatus,
}

impl ExportJob {
    pub fn status_label(&self) -> &'static str {
        match &self.status {
            ExportJobStatus::Queued => "Na fila",
            ExportJobStatus::Rendering { .. } => "Renderizando",
            ExportJobStatus::Paused { .. } => "Pausado",
            ExportJobStatus::Done => "Concluído",
            ExportJobStatus::Failed { .. } => "Falhou",
        }
    }

    pub fn detail_line(&self) -> String {
        match &self.status {
            ExportJobStatus::Failed { message } => format!("Erro: {message}"),
            _ => format!(
                "-{:.0} LUFS · {:.0} Mbps · {}",
                -self.target_lufs, self.bitrate_mbps, self.output_path
            ),
        }
    }
}
