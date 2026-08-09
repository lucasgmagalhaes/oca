use std::fs;
use std::path::Path;
use std::time::Duration;

use nivela_core::proxy::{ensure_proxy, proxy_path_for};

#[test]
fn proxy_path_uses_the_source_stem_with_a_proxy_suffix() {
    let source = Path::new("/videos/boss03_ribby_croaks.mp4");
    let proxy_dir = Path::new("/cache");
    assert_eq!(
        proxy_path_for(source, proxy_dir),
        Path::new("/cache/boss03_ribby_croaks_proxy.mp4")
    );
}

#[test]
fn ensure_proxy_skips_ffmpeg_when_the_proxy_is_already_up_to_date() {
    let dir = std::env::temp_dir().join("nivela_proxy_test_cache_hit");
    fs::create_dir_all(&dir).unwrap();
    let source = dir.join("source.mp4");
    fs::write(&source, b"fake source").unwrap();
    let proxy_path = proxy_path_for(&source, &dir);
    std::thread::sleep(Duration::from_millis(20));
    fs::write(&proxy_path, b"fake proxy").unwrap();

    // If this actually invoked ffmpeg (not installed in this environment), it would return
    // Err(ProxyError::Spawn(_)) instead of Ok - succeeding proves the cache-hit path ran
    // without needing the ffmpeg binary at all.
    let result = ensure_proxy(&source, &dir);

    let _ = fs::remove_file(&source);
    let _ = fs::remove_file(&proxy_path);
    let _ = fs::remove_dir(&dir);

    assert_eq!(result.unwrap(), proxy_path);
}
