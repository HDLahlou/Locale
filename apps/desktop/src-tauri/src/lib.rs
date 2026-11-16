//! Tauri shell for the Locale desktop app. It opens the Vite app.

/// Product name shown in the window title and bundle.
pub const PRODUCT_NAME: &str = "Locale";

/// Starts the Tauri application.
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running Locale");
}

#[cfg(test)]
mod tests {
    #[test]
    fn product_name_is_locale() {
        assert_eq!(super::PRODUCT_NAME, "Locale");
    }
}
