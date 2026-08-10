#[test]
fn gst_init_does_not_panic() {
    gstreamer::init().expect("gstreamer::init() should succeed with GStreamer on PATH");
    assert!(gstreamer::version().0 >= 1);
}
