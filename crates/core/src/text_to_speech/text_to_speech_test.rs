use super::*;

fn fixture_config() -> PiperVoiceConfig {
    let mut phoneme_id_map = HashMap::new();
    phoneme_id_map.insert("_".to_string(), vec![0]);
    phoneme_id_map.insert("^".to_string(), vec![1]);
    phoneme_id_map.insert("$".to_string(), vec![2]);
    phoneme_id_map.insert("a".to_string(), vec![5]);
    phoneme_id_map.insert("t".to_string(), vec![10]);
    // A syllabic consonant — 'n' + U+0329 COMBINING VERTICAL LINE BELOW, one grapheme cluster
    // (unlike a spacing modifier like the stress mark 'ˈ', which is its own separate cluster).
    phoneme_id_map.insert("n\u{0329}".to_string(), vec![20]);

    let mut phoneme_map = HashMap::new();
    phoneme_map.insert("k".to_string(), vec!["t".to_string()]);

    PiperVoiceConfig {
        audio: PiperAudioConfig { sample_rate: 22050 },
        espeak: PiperEspeakConfig {
            voice: "pt-br".to_string(),
        },
        inference: PiperInferenceConfig::default(),
        num_speakers: 1,
        phoneme_map,
        phoneme_id_map,
    }
}

#[test]
fn parses_a_real_piper_voice_config() {
    // Trimmed from a real downloaded pt_BR-faber-medium.onnx.json, verified against Hugging
    // Face before depending on this schema.
    let json = r#"{
        "audio": {"sample_rate": 22050, "quality": "medium"},
        "espeak": {"voice": "pt-br"},
        "inference": {"noise_scale": 0.667, "length_scale": 1, "noise_w": 0.8},
        "phoneme_type": "espeak",
        "phoneme_map": {"c": ["k"]},
        "phoneme_id_map": {"_": [0], "^": [1], "$": [2], " ": [3]},
        "num_speakers": 1
    }"#;
    let config: PiperVoiceConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.audio.sample_rate, 22050);
    assert_eq!(config.espeak.voice, "pt-br");
    assert_eq!(config.num_speakers, 1);
    assert_eq!(config.phoneme_id_map.get("_"), Some(&vec![0]));
    assert_eq!(config.phoneme_map.get("c"), Some(&vec!["k".to_string()]));
}

#[test]
fn parses_config_missing_optional_fields() {
    // No "inference"/"phoneme_map"/"num_speakers" — must still parse via #[serde(default)].
    let json = r#"{
        "audio": {"sample_rate": 16000},
        "espeak": {"voice": "en-us"},
        "phoneme_id_map": {"_": [0]}
    }"#;
    let config: PiperVoiceConfig = serde_json::from_str(json).unwrap();
    assert_eq!(config.num_speakers, 0);
    assert!((config.inference.noise_scale - 0.667).abs() < 1e-6);
    assert!((config.inference.length_scale - 1.0).abs() < 1e-6);
}

#[test]
fn phonemes_to_ids_wraps_with_bos_and_eos() {
    let config = fixture_config();
    let ids = phonemes_to_ids("a", &config);
    assert_eq!(ids, vec![1, 5, 0, 2]); // BOS, 'a', PAD, EOS
}

#[test]
fn phonemes_to_ids_intersperses_pad_between_symbols() {
    let config = fixture_config();
    let ids = phonemes_to_ids("at", &config);
    assert_eq!(ids, vec![1, 5, 0, 10, 0, 2]); // BOS, 'a', PAD, 't', PAD, EOS
}

#[test]
fn phonemes_to_ids_applies_phoneme_map_substitution() {
    let config = fixture_config();
    // 'k' isn't in phoneme_id_map directly, but phoneme_map remaps it to 't'.
    let ids = phonemes_to_ids("k", &config);
    assert_eq!(ids, vec![1, 10, 0, 2]); // BOS, 't' (substituted), PAD, EOS
}

#[test]
fn phonemes_to_ids_skips_unknown_symbols() {
    let config = fixture_config();
    // 'z' has no entry and no substitution — skipped, not an error.
    let ids = phonemes_to_ids("az", &config);
    assert_eq!(ids, vec![1, 5, 0, 2]); // BOS, 'a', PAD, EOS ('z' silently dropped)
}

#[test]
fn phonemes_to_ids_keeps_a_combining_diacritic_with_its_base_char() {
    let config = fixture_config();
    // "n" + U+0329 is one grapheme cluster, one phoneme symbol — must NOT be split into two
    // lookups ('n' alone and the bare combining mark) that would both miss the map.
    let ids = phonemes_to_ids("n\u{0329}", &config);
    assert_eq!(ids, vec![1, 20, 0, 2]);
}

#[test]
fn phonemes_to_ids_empty_input_is_just_bos_and_eos() {
    let config = fixture_config();
    let ids = phonemes_to_ids("", &config);
    assert_eq!(ids, vec![1, 2]);
}

#[test]
fn write_wav_produces_a_readable_file() {
    let dir = std::env::temp_dir().join(format!("oca_tts_wav_test_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("out.wav");
    let samples = vec![0.0f32, 0.5, -0.5, 1.0, -1.0];
    write_wav(&path, 22050, &samples).unwrap();

    let reader = hound::WavReader::open(&path).unwrap();
    let spec = reader.spec();
    assert_eq!(spec.sample_rate, 22050);
    assert_eq!(spec.channels, 1);
    assert_eq!(reader.len(), samples.len() as u32);

    std::fs::remove_file(&path).unwrap();
    std::fs::remove_dir(&dir).unwrap();
}
