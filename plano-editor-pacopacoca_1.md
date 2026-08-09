# Plano de Execução — NivelaEditor (PacoPaçoca)

## Decisões já tomadas

- **Nome:** NivelaEditor.
- **Escopo:** editor completo (timeline, cortes manuais, multi-faixa) — não é uma ferramenta de processamento em lote sem timeline.
- **Diferenciais do projeto:** ajuste automático de áudio (normalização de loudness) e exportação com bitrate igual ao da fonte.
- **Linguagem:** Rust, apoiado em GStreamer (`gstreamer-rs`) ou MLT Framework para o motor de decode/render, evitando reescrever o pipeline de mídia do zero.
- **Novo requisito:** fila de exportação em background — dá pra continuar editando ou exportando outros itens enquanto uma exportação está rodando.

---

## Fase 0 — Escopo do MVP

Definir o corte mínimo antes de codar: import, timeline com faixas V1/A1/A2, corte/split/trim, preview, normalização automática no export, bitrate travado na fonte, fila de exportação. Efeitos avançados, plugins e correção de cor ficam pra depois.

## Fase 1 — Motor central

- Wrapper de `ffprobe` para extrair codec, bitrate, LUFS, fps e resolução de cada clipe importado.
- Pipeline de decode/preview via GStreamer (ou MLT), rodando independente da UI.
- Modelo de projeto em memória (clipes, faixas, timeline) + formato de salvamento em JSON.

## Fase 2 — Ajuste automático de áudio

- Análise de loudness integrada (LUFS) por clipe.
- Normalização em duas passadas (equivalente ao filtro `loudnorm` do FFmpeg), com perfis prontos (ex.: −14 LUFS pra YouTube).
- Limitador de picos (true peak) pra evitar clipping depois da normalização.
- Aplicada automaticamente antes da exportação, sem exigir ajuste manual clipe a clipe.

## Fase 3 — Timeline e edição

- Widget de timeline custom: waveform nos clipes de áudio, thumbnail nos de vídeo.
- Interações de corte, split, trim e arraste entre faixas.
- Player de preview sincronizado ao playhead, com scrubbing fluido (por isso GUI nativa, não webview).

## Fase 4 — Exportação e fila em background

Esta é a fase que muda mais com o novo requisito. A ideia central: **renderização nunca pode travar a edição.**

**Arquitetura**
- A renderização roda num worker separado da thread de UI (processo ou thread dedicada), comunicando por canal assíncrono (ex.: `tokio::mpsc` em Rust). A interface de edição continua 100% responsiva enquanto um job renderiza.
- Cada exportação vira um **job** independente, com um snapshot das configurações no momento em que entra na fila: bitrate alvo, perfil de áudio, formato, caminho de saída. Mudanças feitas depois no projeto ativo não afetam jobs já enfileirados.

**Fila**
- Painel de fila mostrando todos os jobs: status (na fila / renderizando / concluído / falhou), progresso individual, nome do projeto/sequência, destino do arquivo.
- Reordenar, pausar e cancelar jobs individualmente.
- Fila persiste entre sessões — útil pra deixar exportações rodando de um dia pro outro.

**Concorrência**
- Por padrão, 1 worker de render em segundo plano — evita que a exportação dispute CPU/GPU com o preview em tempo real da edição ativa.
- Número de workers simultâneos configurável nas preferências, pra quem tiver hardware de sobra.

**Caso de uso direto:** séries de cortes como a dos 50 chefes do Cuphead — dá pra enfileirar vários clipes pra exportação e seguir cortando o próximo enquanto os anteriores renderizam em background.

## Fase 5 — Robustez

- Autosave e recuperação de projeto (essencial em sessões de horas).
- Atalhos de teclado configuráveis.
- Preferências gerais (perfis de áudio padrão, número de workers de export, pasta de saída padrão).

## Fase 6 — Empacotamento

- Build e instalador para Windows (ambiente atual de produção).
- Testes com material real do canal: gameplay longo (Minecraft) e clipes curtos (cortes).

---

## Stack técnica resumida

| Camada | Escolha |
|---|---|
| Linguagem | Rust |
| Motor de mídia | GStreamer (`gstreamer-rs`) ou MLT Framework |
| GUI | `egui` / `iced` / `Slint` (nativa, não webview) |
| Concorrência da fila | `tokio` + canais assíncronos |
| Probe/encode final | `ffprobe` / `ffmpeg` |
| Formato de projeto | JSON |
