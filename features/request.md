# Plano de Execução — oca (PacoPaçoca)

## Decisões já tomadas

- **Nome:** oca.
- **Escopo:** editor completo (timeline, cortes manuais, multi-faixa) — não é uma ferramenta de processamento em lote sem timeline.
- **Diferenciais do projeto:** ajuste automático de áudio (normalização de loudness) e exportação com bitrate igual ao da fonte.
- **Linguagem:** Rust, apoiado em GStreamer (`gstreamer-rs`) ou MLT Framework para o motor de decode/render, evitando reescrever o pipeline de mídia do zero.
- **Ponte nativa em C:** probe e encode falam com libav (FFmpeg) via uma biblioteca C própria, ligada ao Rust por FFI — sem spawnar `ffprobe`/`ffmpeg` como processo externo.
- **Tudo embutido:** FFmpeg/libav, GStreamer, o modelo do Whisper e o motor de TTS vêm empacotados no instalador — nada disso é baixado ou instalado à parte pelo usuário.
- **Plataformas:** Windows (ambiente atual de produção) e Linux.
- **Fila de exportação em background:** continua editando ou exportando outros itens enquanto uma exportação está rodando.
- **Performance e leveza:** requisito do projeto, não ajuste posterior — guia decisões como GUI nativa (sem webview), proxy de edição pra material pesado e encode acelerado por GPU (ver Fase 7).

---

## Fase 0 — Escopo do MVP

Definir o corte mínimo antes de codar: import, timeline com faixas V1/A1/A2, corte/split/trim, preview, normalização automática no export, bitrate travado na fonte, fila de exportação. Plugins de terceiros e correção de cor avançada ficam fora do MVP — os efeitos e ferramentas da Fase 4 entram depois que o fluxo básico de corte + áudio + exportação estiver funcionando.

## Fase 1 — Motor central

- **Ponte C própria (`oca-avbridge`):** biblioteca C fina que embrulha libavformat/libavcodec — expõe funções simples (`extern "C"`) pra probe de metadata (codec, bitrate, LUFS, fps, resolução) e pro encode final. Rust chama direto via FFI, sem subprocesso e sem parsear texto de stdout. Como é código C escrito pra esse projeto (não um binding gerado automaticamente de toda a API do FFmpeg), a superfície fica pequena e fácil de auditar.
- Pipeline de decode/preview via GStreamer (ou MLT), rodando independente da UI — esse aqui continua via binding direto (`gstreamer-rs`), sem subprocesso também.
- Modelo de projeto em memória (clipes, faixas, timeline) + formato de salvamento em JSON.

## Fase 2 — Ajuste automático de áudio

- Análise de loudness integrada (LUFS) por clipe.
- Normalização em duas passadas (equivalente ao filtro `loudnorm` do FFmpeg), com perfis prontos (ex.: −14 LUFS pra YouTube).
- **Redução de ruído:** remoção automática de ruído de fundo/chiado do áudio, aplicada junto da normalização (recurso presente no CapCut e que não estava no plano).
- Limitador de picos (true peak) pra evitar clipping depois da normalização.
- Aplicada automaticamente antes da exportação, sem exigir ajuste manual clipe a clipe.

## Fase 3 — Timeline, edição e organização do projeto

- Widget de timeline custom: a faixa de vídeo mostra thumbnails de cada frame do clipe, recalculados conforme o zoom (mais zoom = mais frames visíveis individualmente, menos zoom = thumbnails mais espaçados); logo abaixo, uma faixa separada com a variação de volume do áudio (waveform), sincronizada ao mesmo nível de zoom — mesmo padrão usado em editores como o CapCut.
- Interações de corte, split, trim e arraste entre faixas.
- Player de preview sincronizado ao playhead, com scrubbing fluido (por isso GUI nativa, não webview).
- Zoom na timeline — aproximar pra editar com precisão frame a frame, afastar pra ver o projeto inteiro. Controlado por `Ctrl` + scroll do mouse.
- **Abas de projeto:** múltiplas sequências dentro do mesmo projeto (ex.: uma aba pra cortes, outra pro vídeo completo), cada uma com sua própria timeline e configurações de export.
- **Painéis de UI redimensionáveis:** divisórias entre biblioteca, preview, propriedades e timeline arrastáveis pelo mouse, com o layout salvo por projeto ou por usuário.
- **Blocos compostos:** opção de mesclar vários cortes selecionados em um único bloco composto, que pode ser movido, cortado e reutilizado na timeline como se fosse um clipe só.
- **Copiar e colar entre abas:** blocos de vídeo, áudio ou efeito podem ser copiados e colados na mesma aba ou em outra aba do projeto.
- **Menu de contexto:** clique com o botão direito num bloco da timeline abre um menu com as mesmas ações dos atalhos (copiar, colar, recortar, copiar formatação, aplicar efeito, mesclar em bloco composto, etc.) — cobre quem não decorou os atalhos.

## Fase 4 — Ferramentas e efeitos de edição

- **Efeitos visuais:** biblioteca de efeitos aplicáveis por clipe, com intensidade ajustável. Conjunto inicial proposto:
  - Blur e shake (tremido de câmera)
  - Zoom (punch-in / ken burns)
  - Brilho, contraste e saturação
  - Preto e branco e sépia
  - Vinheta
  - Espelhar (flip horizontal)
  - Chroma key (remoção de fundo verde — útil pro recorte de webcam)
  - Glitch
  - Nitidez (sharpen)
  - Pixelizar/censura (mosaico)
  - Transições entre clipes (fade, corte seco, slide, zoom)
- **Texto:** caixa de texto sobre o vídeo com formatação de fonte, tamanho, cor e background configuráveis. Categorias de fonte propostas (fontes livres, tipo Google Fonts, embutidas no app pra funcionar igual em qualquer máquina):
  - Sans-serif moderna (ex.: Montserrat, Poppins, Inter)
  - Display/impacto pra títulos (ex.: Bebas Neue, Anton, Impact)
  - Serifada (ex.: Playfair Display)
  - Manuscrita/casual (ex.: Caveat)
  - Monoespaçada (ex.: JetBrains Mono, Roboto Mono)
  - Bold de alto contraste pra legenda de shorts (ex.: Montserrat Black/ExtraBold, Poppins Black, Archivo Black) — a família usada nos estilos de legenda com destaque de palavra abaixo.
- **Legendas automáticas:** transcrição automática do áudio (reconhecimento de fala com suporte a PT-BR, ex.: Whisper) gerando blocos de legenda sincronizados e editáveis — texto, tempo e estilo ajustáveis depois de gerados. Exportável como texto embutido no vídeo ou como arquivo `.srt` separado.
- **Legenda com destaque de palavra (estilo shorts):** usa os timestamps por palavra do Whisper pra destacar/colorir cada palavra exatamente no momento em que é falada — o estilo popular em shorts (tipo MrBeast/Hormozi), com fundo (box) atrás do texto configurável junto do resto da formatação.
- **Templates de grupo de camadas:** um grupo de camadas configurado na timeline (ex.: webcam recortada + fundo com blur + jogo centralizado) pode ser salvo como template, guardando posição, escala, crop e efeitos de cada camada. Ao aplicar o template num short novo, o editor recria as mesmas camadas com as mesmas configs de uma vez — só pedindo os clipes de origem pra cada camada, sem precisar copiar/colar e reconfigurar efeito por efeito toda vez.
- **Copiar formatação:** copia só as configurações/efeitos aplicados a um bloco (vídeo, áudio ou efeito) e cola em outro bloco, sem duplicar o clipe em si — atalhos `Ctrl+Shift+C` (copiar) e `Ctrl+Shift+V` (colar).
- **Múltiplas camadas:** a timeline suporta várias camadas de vídeo/áudio sobrepostas, não só faixas sequenciais — necessário pra overlays, picture-in-picture e composições (base pros templates de camadas acima).
- **Transformação de camadas:** largura, altura e posição de cada bloco de vídeo ajustáveis dentro do quadro de duas formas equivalentes — direto no preview, arrastando o bloco e suas alças de redimensionamento com o mouse, ou por valores numéricos no menu de configuração do bloco. As duas formas alteram a mesma transformação e ficam sincronizadas.
- **Keyframes:** sistema geral de quadros-chave, não só pra opacidade — posição, escala, rotação e opacidade podem receber marcadores em pontos diferentes do clipe, e o valor é interpolado automaticamente entre eles (mesmo conceito usado no CapCut pra animar qualquer propriedade). O caso de opacidade por dois marcadores descrito antes é uma aplicação direta desse sistema.
- **Velocidade:** ajuste de velocidade do vídeo (câmera lenta, acelerado), com reamostragem de áudio compatível com a normalização automática da Fase 2.
- **Recorte (crop):** ferramenta de corte do quadro/reenquadramento, separada do corte de tempo (split) já coberto na Fase 3.
- **Congelar:** freeze frame — segura um quadro específico por uma duração configurável.
- **Ganho de volume por bloco:** aumenta ou diminui o volume do áudio de um bloco de vídeo ou de um bloco de áudio, individualmente. A waveform daquele bloco na timeline reflete o ajuste na hora — barras mais altas pro ganho positivo, mais baixas pra atenuação — mesmo mecanismo visual da faixa de áudio já prevista na Fase 3.
- **Máscaras:** recorte de camada em formatos (círculo, retângulo arredondado, forma customizada), além do crop retangular simples — útil pra molduras de webcam.
- **Estabilização de vídeo:** reduz tremido de câmera em material já gravado — o oposto do efeito de shake (que adiciona tremido de propósito).
- **Remoção de fundo por IA:** corta a pessoa do fundo sem precisar de tela verde, complementando o chroma key (que depende de fundo verde/croma).
- **Reenquadramento automático:** ao trocar a proporção de export (16:9, 9:16 etc.), a IA recentraliza o assunto principal no novo quadro automaticamente, com opção de ajuste manual por cima.
- **Filtros de cor e LUTs:** biblioteca de filtros prontos e suporte a LUTs, além dos ajustes básicos de brilho/contraste/saturação já previstos.
- **Texto-pra-fala:** gera narração sintética a partir de um texto digitado.
- **Rastreamento de movimento:** anexa texto ou efeito a um ponto que se move na cena (ex.: seguir um objeto ou rosto).
- **Biblioteca de música e efeitos sonoros:** trilhas e SFX prontos pra usar direto no projeto, sem precisar importar de fora.
- **Remoção de flicker:** corrige cintilação de vídeo, comum em gravações de tela/gameplay sob certas taxas de atualização.

Efeitos e composição em tempo real (blur, shake, camadas, opacidade) pedem aceleração por GPU — ver nota na stack técnica.

Os itens de máscaras até remoção de flicker acima foram adicionados após comparar o plano com os recursos do CapCut Desktop — cobrem lacunas reais (estabilização, reenquadramento automático, remoção de fundo por IA, LUTs, text-to-speech, motion tracking, bibliotecas de música/SFX). O AI Auto-Edit do CapCut (edição automática a partir de um prompt) não entrou: é um projeto de IA generativa à parte, bem maior que o resto do escopo.

## Fase 5 — Exportação e fila em background

Renderização nunca pode travar a edição.

**Arquitetura**
- A renderização roda num worker separado da thread de UI (processo ou thread dedicada), comunicando por canal assíncrono (ex.: `tokio::mpsc` em Rust). A interface de edição continua 100% responsiva enquanto um job renderiza.
- Cada exportação vira um **job** independente, com um snapshot das configurações no momento em que entra na fila: bitrate alvo, perfil de áudio, formato, proporção de tela, caminho de saída. Mudanças feitas depois no projeto ativo não afetam jobs já enfileirados.

**Fila**
- Painel de fila mostrando todos os jobs: status (na fila / renderizando / concluído / falhou), progresso individual, nome do projeto/sequência, destino do arquivo.
- Reordenar, pausar e cancelar jobs individualmente.
- Fila persiste entre sessões — útil pra deixar exportações rodando de um dia pro outro.

**Concorrência**
- Por padrão, 1 worker de render em segundo plano — evita que a exportação dispute CPU/GPU com o preview em tempo real da edição ativa.
- Número de workers simultâneos configurável nas preferências, pra quem tiver hardware de sobra.

**Configurações de export**
- **Proporção/dimensão de tela:** seleção de aspect ratio pra exportar (16:9, 9:16, 1:1 e outras), refletida ao vivo no preview antes de exportar.
- **Preview de tamanho do arquivo:** estimativa do tamanho final atualizada conforme bitrate, duração e formato são ajustados, antes de confirmar a exportação.
- **Encode por GPU:** opção de usar aceleração de hardware no encode final (NVENC na Nvidia, Quick Sync na Intel, AMF na AMD), acionada pela ponte C (`oca-avbridge`) da Fase 1 — com fallback pro encode por CPU quando a GPU não suportar o codec escolhido.
- **Local e nome do arquivo de exportação:** pasta de destino e nome do arquivo escolhidos diretamente no painel de export (além da pasta padrão configurável nas preferências). Verifica se já existe um arquivo com esse nome no destino antes de exportar — se existir, avisa e deixa escolher entre sobrescrever, renomear automaticamente (ex.: sufixo numérico) ou cancelar.

**Caso de uso direto:** séries de cortes como a dos 50 chefes do Cuphead — dá pra enfileirar vários clipes pra exportação e seguir cortando o próximo enquanto os anteriores renderizam em background.

## Fase 6 — Robustez

- **Autosave com debounce:** salva automaticamente a cada alteração no projeto, com debounce de 2 segundos — o timer reseta a cada nova edição, então só grava depois de 2s sem mudanças. Um teto máximo (ex.: a cada 30s) força um save mesmo em sessões de edição contínua, pra nunca passar tempo demais sem persistir. Grava em background thread pra não travar a UI, num arquivo de recuperação separado do save manual — se o app fechar sem um save explícito, oferece restaurar a partir do autosave ao reabrir.
- **Key bindings configuráveis:** mapeamento de atalhos editável pelo usuário. Padrões definidos:
  - `Espaço` — play/pause no preview
  - `Ctrl+B` — cortar o vídeo (split)
  - `Ctrl+O` — adicionar marcador de opacidade
  - `Ctrl+Shift+C` — copiar formatação de um bloco
  - `Ctrl+Shift+V` — colar formatação em outro bloco
- **Modal de configurações:** a tela de configurações (incluindo os key bindings) abre como uma modal centralizada, sobreposta dentro da própria janela do app — não uma janela do sistema operacional separada nem uma tela/rota própria — com largura pequena, ocupando só o espaço necessário pro conteúdo.
- **Tratamento de erros:** nenhum ponto de risco (leitura de arquivo, decode, encode, chamadas ao FFmpeg/GStreamer) falha silenciosamente. Em Rust isso é `Result<T, E>` com tipos de erro próprios propagados até um handler central, que decide se mostra um aviso pro usuário, tenta recuperar sozinho, ou registra e segue — o equivalente ao padrão try/catch de outras linguagens, mas explícito no tipo de cada função.
- **Logs:** log estruturado em níveis (erro, aviso, info, debug), gravado em arquivo local rotativo, com contexto suficiente pra reproduzir um problema depois (versão do app, ação em andamento, stack trace quando houver).
- **Backup em crash:** se o app travar, o autosave mais recente é preservado e, ao reabrir, o app detecta o fechamento anormal e oferece restaurar o projeto de onde parou.
- **Relatório de crash:** panics não tratados são capturados com stack trace e salvos localmente — essa base também alimenta a telemetria da Fase 7, pra identificar padrões de falha recorrentes.
- Preferências gerais (perfis de áudio padrão, número de workers de export, pasta de saída padrão, layout de painéis padrão).

## Fase 7 — Performance e leveza

Performance é tratada como requisito, não como ajuste fino de última hora — várias decisões anteriores já foram tomadas pensando nisso (GUI nativa sem webview, efeitos via GPU, fila de exportação em worker separado). Esta fase reúne o resto:

- **Proxy de edição:** para material pesado (4K/2h), o preview usa uma cópia de baixa resolução (proxy) gerada na importação, mantendo a edição fluida; a exportação final sempre usa o arquivo original em qualidade cheia.
- **Qualidade do preview selecionável:** menu de qualidade de visualização (ex.: 360p/480p/720p), com teto em 720p — o suficiente pra avaliar enquadramento, texto e cor sem forçar o preview a decodificar em resolução total e prejudicar o desempenho da edição. Usa o mesmo proxy do item acima; a exportação final nunca é afetada por essa escolha.
- **Decode acelerado por hardware no preview:** não só no encode de exportação — usar VideoToolbox/VAAPI/NVDEC/Quick Sync também pra decodificar durante o preview e scrubbing, tirando carga da CPU. Ativo por padrão quando houver decoder compatível, com fallback automático para CPU e uma opção nas preferências para forçar decode por software.
- **Carregamento preguiçoso:** só decodifica/mantém em memória os frames próximos do playhead, não o clipe inteiro.
- **Formato de projeto leve:** o JSON do projeto guarda referências aos arquivos originais, nunca copia mídia pra dentro do projeto.
- **Build de release otimizado:** LTO, `opt-level=3` e strip de símbolos no binário final, pra reduzir tamanho do executável e tempo de inicialização.
- **Métricas-alvo mensuráveis:** definir e acompanhar números concretos ao longo do desenvolvimento — tempo de import, frame time no scrubbing, uso de RAM numa sessão de 2h, tempo de export por minuto de vídeo — em vez de tratar "leveza" como algo subjetivo.
- **Telemetria de runtime:** coleta estruturada de métricas de uso real (frame time do preview, duração de import/export, uso de CPU/RAM/GPU, eventos de erro), gravada localmente em formato estruturado (ex.: JSON lines). Fica no dispositivo por padrão — a ideia é ter dado histórico real pra guiar otimizações futuras, não achismo. Reaproveita a mesma base de logs/eventos da Fase 6.

## Fase 8 — Empacotamento

> **Status:** bundles portáteis automatizados para Windows e Linux/AppImage concluídos. Todos os
> modelos e runtimes listados abaixo são montados no CI com SHA-256 e validados antes da
> publicação; o app não baixa modelos sob demanda. O instalador Inno Setup para Windows permite
> escolher a pasta. Pendentes: pacote `.deb` e encode VAAPI.

- **Build e instalador para Windows e Linux.** GUI nativa (`egui`/`iced`/`Slint`) e GStreamer já são multiplataforma por natureza, então a maior parte do trabalho extra fica no empacotamento, não no código do app em si.
  - Windows: instalador `.msi`/`.exe`.
  - Linux: **AppImage** como formato principal (roda em qualquer distro sem instalar nada do sistema, mais parecido com "baixou, rodou" do Windows); pacote `.deb` como alternativa pra quem prefere instalar via gerenciador de pacotes.
- **Motores embutidos no instalador:** nenhum motor nativo precisa ser baixado ou instalado à parte pelo usuário — vale pras duas plataformas.
  - libav (FFmpeg) — linkado estático na ponte C (`oca-avbridge`), viaja dentro do próprio executável.
  - GStreamer — os plugins usados (não o framework inteiro) empacotados junto do instalador, ao lado do executável.
  - Modelo do Whisper e motor de TTS — arquivos de modelo incluídos no instalador, funcionando offline desde a primeira abertura.
  - ONNX Runtime (remoção de fundo/reenquadramento) — biblioteca redistribuível empacotada junto, mesma lógica.
  - No Linux, o AppImage empacota essas bibliotecas junto do binário, evitando depender de versões instaladas no sistema (que variam muito de distro pra distro).
- **Encode por GPU no Linux:** o caminho comum é VAAPI (Intel/AMD) — a mesma ponte C que já cobre NVENC/Quick Sync/AMF no Windows.
- **Local de instalação configurável:** o instalador permite escolher a pasta onde o app é instalado, em vez de um caminho fixo.
- **Versão e auto-update:** um "Sobre" mostra a versão atual instalada. O app consulta a última release publicada no repositório do projeto (GitHub Releases) e, se houver versão mais nova, avisa e oferece baixar e aplicar a atualização — nas duas plataformas.
- Testes com material real do canal: gameplay longo (Minecraft) e clipes curtos (cortes).

---

## Stack técnica resumida

| Camada | Escolha |
|---|---|
| Linguagem | Rust |
| Motor de mídia (decode/preview) | GStreamer via `gstreamer-rs` — lib nativa (bindings FFI), não subprocesso |
| Probe de metadata | ponte C própria (`oca-avbridge`) sobre libavformat, linkada estática, chamada via FFI — sem subprocesso |
| Encode final (exportação) | mesma ponte C (`oca-avbridge`) sobre libavcodec/NVENC/Quick Sync/AMF — sem subprocesso |
| GUI | `egui` / `iced` / `Slint` (nativa, não webview) |
| Efeitos/composição em tempo real | `wgpu` (Vulkan/Metal/DX12) para blur, shake, camadas e opacidade acelerados por GPU |
| Legendas automáticas | Whisper (reconhecimento de fala local, com suporte a PT-BR) |
| Remoção de fundo por IA / reenquadramento automático | modelo de segmentação/detecção rodando local (ex.: via `ort`/ONNX Runtime em Rust) |
| Texto-pra-fala | motor de TTS com suporte a PT-BR (a definir) |
| Logs, telemetria e captura de crash | crate `tracing` (Rust) com saída em arquivo rotativo + hook de panic customizado |
| Auto-update | crate `self_update` (Rust), consultando a API de releases do GitHub |
| Concorrência da fila | `tokio` + canais assíncronos |
| Formato de projeto | JSON |
| Empacotamento de motores | libav estático na ponte C; plugins do GStreamer, modelo do Whisper, motor de TTS e ONNX Runtime empacotados no instalador |
