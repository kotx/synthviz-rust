use rfd::{AsyncFileDialog, FileHandle};

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
}

pub fn init_log() {
    #[cfg(target_arch = "wasm32")]
    console_log::init_with_level(log::Level::Info).unwrap();
    #[cfg(not(target_arch = "wasm32"))]
    pretty_env_logger::init();
}

pub async fn pick_file() -> Option<FileHandle> {
    AsyncFileDialog::new()
        .add_filter("audio", &["mp3", "ogg", "flac", "wav", "pcm"])
        .pick_file()
        .await
}
