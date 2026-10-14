fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "snapshot",
            "pick_folder",
            "new_session",
            "select_session",
            "set_workflow",
            "submit_task",
            "cancel_run",
            "decide_permission",
            "history",
            "subscribe",
            "acknowledge",
            "subscription_status",
            "sign_in",
            "material_status",
            "quit_application",
        ]),
    ))
    .expect("desktop capability manifest failed");
}
