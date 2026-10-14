//! Tauri adapter: local view authorization, native presentation and host lifecycle.
mod material;
mod settings;
use astrid::{
    auth,
    cancellation::Cancellation,
    events::{RunId, SessionId, ToolCallId},
    host::{
        Host, Receipt, Submit,
        view::{Batch, HistoryPage, Snapshot, Workflow},
    },
    openai::OpenAiProvider,
};
use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
use tauri::{AppHandle, Manager, State, WebviewWindow, ipc::Channel};
use uuid::Uuid;
struct Desktop {
    host: Arc<Host>,
    directory: PathBuf,
    auth_phase: Arc<Mutex<String>>,
    client_error: Arc<Mutex<Option<String>>>,
    restoring: Arc<AtomicBool>,
    login_cancel: Cancellation,
    quitting: Arc<AtomicBool>,
    exit_ready: Arc<AtomicBool>,
    admission: Mutex<()>,
}
#[derive(Serialize)]
struct DesktopSnapshot {
    #[serde(flatten)]
    host: Snapshot,
    auth_phase: String,
    client_error: Option<String>,
    restoring: bool,
}
fn local_origin(label: &str, url: &tauri::Url) -> bool {
    label == "main"
        && url.scheme() == "tauri"
        && url.host_str() == Some("localhost")
        && matches!(url.path(), "" | "/" | "/index.html")
        && url.query().is_none()
        && url.fragment().is_none()
}
fn trusted(window: &WebviewWindow) -> Result<(), String> {
    let url = window.url().map_err(|_| "Invoking view unavailable")?;
    if local_origin(window.label(), &url) {
        Ok(())
    } else {
        Err("Unauthorized invoking view".into())
    }
}
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| "Host operation failed".to_string())?
}
#[tauri::command]
async fn snapshot(
    window: WebviewWindow,
    state: State<'_, Desktop>,
) -> Result<DesktopSnapshot, String> {
    trusted(&window)?;
    let host = state.host.clone();
    let phase = state
        .auth_phase
        .lock()
        .map_err(|_| "Sign-in status unavailable")?
        .clone();
    let client_error = state
        .client_error
        .lock()
        .map_err(|_| "Client status unavailable")?
        .clone();
    let restoring = state.restoring.load(Ordering::SeqCst);
    let snapshot = blocking(move || host.snapshot("main").map_err(|e| e.to_string())).await?;
    let result = DesktopSnapshot {
        host: snapshot,
        auth_phase: phase,
        client_error,
        restoring,
    };
    if serde_json::to_vec(&result)
        .map_err(|_| "Projection failed")?
        .len()
        > 64 * 1024
    {
        return Err("Current-state projection exceeded its byte cap".into());
    }
    Ok(result)
}
#[tauri::command]
async fn pick_folder(
    window: WebviewWindow,
    app: AppHandle,
    state: State<'_, Desktop>,
) -> Result<(), String> {
    trusted(&window)?;
    if state.restoring.load(Ordering::SeqCst) {
        return Err("Folder restoration in progress".into());
    }
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.run_on_main_thread(move || {
        #[cfg(target_os = "macos")]
        let folder = {
            use objc2::MainThreadMarker;
            use objc2_app_kit::NSOpenPanel;
            MainThreadMarker::new().and_then(|mtm| {
                let panel = NSOpenPanel::openPanel(mtm);
                panel.setCanChooseFiles(false);
                panel.setCanChooseDirectories(true);
                panel.setAllowsMultipleSelection(false);
                if panel.runModal() == 1 {
                    panel
                        .URL()
                        .and_then(|url| url.path())
                        .map(|path| PathBuf::from(path.to_string()))
                } else {
                    None
                }
            })
        };
        #[cfg(not(target_os = "macos"))]
        let folder: Option<PathBuf> = None;
        let _ = tx.send(folder);
    })
    .map_err(|_| "Folder picker unavailable")?;
    if let Some(folder) = rx.await.map_err(|_| "Folder picker closed")? {
        let host = state.host.clone();
        let directory = state.directory.clone();
        let status = state.client_error.clone();
        blocking(move || {
            host.open_folder("main", &folder)
                .map_err(|e| e.to_string())?;
            let result = settings::save(&directory, &folder).map_err(|_| {
                "Folder selected, but remembering it for next launch failed".to_string()
            });
            if let Ok(mut status) = status.lock() {
                *status = result.as_ref().err().cloned();
            }
            result
        })
        .await?;
    }
    Ok(())
}
#[tauri::command]
async fn new_session(
    window: WebviewWindow,
    state: State<'_, Desktop>,
) -> Result<SessionId, String> {
    trusted(&window)?;
    let host = state.host.clone();
    blocking(move || host.new_session("main").map_err(|e| e.to_string())).await
}
#[tauri::command]
async fn select_session(
    window: WebviewWindow,
    state: State<'_, Desktop>,
    id: SessionId,
) -> Result<(), String> {
    trusted(&window)?;
    let host = state.host.clone();
    blocking(move || host.select_session("main", id).map_err(|e| e.to_string())).await
}
#[tauri::command]
async fn set_workflow(
    window: WebviewWindow,
    state: State<'_, Desktop>,
    workflow: Workflow,
) -> Result<(), String> {
    trusted(&window)?;
    state
        .host
        .workflow("main", workflow)
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn submit_task(
    window: WebviewWindow,
    state: State<'_, Desktop>,
    intent: Submit,
) -> Result<Receipt, String> {
    trusted(&window)?;
    let _admission = state
        .admission
        .lock()
        .map_err(|_| "Host admission unavailable")?;
    if state.restoring.load(Ordering::SeqCst)
        || state
            .auth_phase
            .lock()
            .map_err(|_| "Sign-in state unavailable")?
            .as_str()
            == "signing_in"
    {
        return Err("Wait for folder restoration or sign-in to finish".into());
    }
    state.host.submit("main", intent).map_err(|e| e.to_string())
}
#[tauri::command]
async fn cancel_run(
    window: WebviewWindow,
    state: State<'_, Desktop>,
    request: Uuid,
) -> Result<(), String> {
    trusted(&window)?;
    state
        .host
        .cancel("main", request)
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn decide_permission(
    window: WebviewWindow,
    state: State<'_, Desktop>,
    id: Uuid,
    run: RunId,
    tool: ToolCallId,
    allow: bool,
) -> Result<(), String> {
    trusted(&window)?;
    state
        .host
        .permission("main", id, run, tool, allow)
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn history(
    window: WebviewWindow,
    state: State<'_, Desktop>,
    offset: usize,
) -> Result<HistoryPage, String> {
    trusted(&window)?;
    state
        .host
        .history("main", offset)
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn subscribe(
    window: WebviewWindow,
    state: State<'_, Desktop>,
    after: u64,
    channel: Channel<Batch>,
) -> Result<Uuid, String> {
    trusted(&window)?;
    let id = state
        .host
        .attach("main", after)
        .map_err(|e| e.to_string())?;
    let host = state.host.clone();
    tauri::async_runtime::spawn(async move {
        loop {
            if !host.attached("main", id).unwrap_or(false) {
                break;
            }
            if let Ok(batch) = host.batch("main", id, None)
                && channel.send(batch).is_err()
            {
                let _ = host.detach("main", id);
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    });
    Ok(id)
}
#[tauri::command]
async fn acknowledge(
    window: WebviewWindow,
    state: State<'_, Desktop>,
    id: Uuid,
    batch: u64,
) -> Result<(), String> {
    trusted(&window)?;
    state
        .host
        .acknowledge("main", id, batch)
        .map_err(|e| e.to_string())
}
#[tauri::command]
async fn subscription_status(
    window: WebviewWindow,
    state: State<'_, Desktop>,
    id: Uuid,
) -> Result<bool, String> {
    trusted(&window)?;
    state.host.attached("main", id).map_err(|e| e.to_string())
}
#[tauri::command]
async fn material_status(window: WebviewWindow) -> Result<material::Material, String> {
    trusted(&window)?;
    material::update(&window).await
}
#[tauri::command]
async fn sign_in(window: WebviewWindow, state: State<'_, Desktop>) -> Result<(), String> {
    trusted(&window)?;
    let _admission = state
        .admission
        .lock()
        .map_err(|_| "Host admission unavailable")?;
    if state.quitting.load(Ordering::SeqCst)
        || state
            .host
            .snapshot("main")
            .map_err(|e| e.to_string())?
            .run
            .is_some_and(|r| {
                matches!(
                    r.status.as_str(),
                    "running" | "starting" | "waiting_permission"
                )
            })
    {
        return Err("Sign-in is available between runs".into());
    }
    {
        let mut phase = state.auth_phase.lock().map_err(|_| "Sign-in unavailable")?;
        if phase.as_str() == "signing_in" {
            return Err("Sign-in already in progress".into());
        }
        *phase = "signing_in".into();
    }
    let directory = state.directory.clone();
    let phase = state.auth_phase.clone();
    let cancel = state.login_cancel.clone();
    tauri::async_runtime::spawn(async move {
        let result = tokio::select! {
            result=auth::login(&directory,|url|{
                let parsed=tauri::Url::parse(url).map_err(std::io::Error::other)?;
                if parsed.scheme()!="https"||parsed.host_str()!=Some("auth.openai.com"){return Err(std::io::Error::other("unexpected sign-in destination"));}
                std::process::Command::new("open").arg(url).spawn().map(|_|())
            })=>result.is_ok(),
            _=cancel.cancelled()=>false,
        };
        if let Ok(mut phase) = phase.lock() {
            *phase = if result { "signed_in" } else { "failed" }.into();
        }
    });
    Ok(())
}
#[tauri::command]
async fn quit_application(window: WebviewWindow, app: AppHandle) -> Result<(), String> {
    trusted(&window)?;
    app.exit(0);
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = auth::default_directory()?;
    let provider = Arc::new(OpenAiProvider::new(Arc::new(auth::ChatGptAuth::new(
        directory.clone(),
    )?))?);
    let host = Host::new(
        provider,
        directory.clone(),
        std::env::var("ASTRID_MODEL").unwrap_or_else(|_| "gpt-5.6-sol".into()),
    )?;
    let desktop = Desktop {
        host,
        directory,
        auth_phase: Arc::new(Mutex::new("existing_unverified".into())),
        client_error: Arc::new(Mutex::new(None)),
        restoring: Arc::new(AtomicBool::new(true)),
        login_cancel: Cancellation::default(),
        quitting: Arc::new(AtomicBool::new(false)),
        exit_ready: Arc::new(AtomicBool::new(false)),
        admission: Mutex::new(()),
    };
    let app = tauri::Builder::default()
        .manage(desktop)
        .invoke_handler(tauri::generate_handler![
            snapshot,
            pick_folder,
            new_session,
            select_session,
            set_workflow,
            submit_task,
            cancel_run,
            decide_permission,
            history,
            subscribe,
            acknowledge,
            subscription_status,
            sign_in,
            material_status,
            quit_application
        ])
        .plugin(
            tauri::plugin::Builder::<tauri::Wry, ()>::new("navigation-boundary")
                .on_navigation(|view, url| local_origin(view.label(), url))
                .build(),
        )
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .setup(|app| {
            let state = app.state::<Desktop>();
            let host = state.host.clone();
            let directory = state.directory.clone();
            let status = state.client_error.clone();
            let restoring = state.restoring.clone();
            tauri::async_runtime::spawn(async move {
                let result = blocking(move || {
                    let folder = std::env::var_os("ASTRID_DESKTOP_FOLDER")
                        .map(PathBuf::from)
                        .map(|path| Ok(Some(path)))
                        .unwrap_or_else(|| {
                            settings::load(&directory).map_err(|_| {
                                "Saved folder selection is invalid or unavailable".to_string()
                            })
                        });
                    if let Some(path) = folder? {
                        host.open_folder("main", &path).map_err(|e| e.to_string())?;
                    }
                    Ok(())
                })
                .await;
                if let Ok(mut status) = status.lock() {
                    *status = result.err();
                }
                restoring.store(false, Ordering::SeqCst);
            });
            Ok(())
        })
        .build(tauri::generate_context!())?;
    app.run(|app,event|match event {
        tauri::RunEvent::ExitRequested {api,..}=>{
            let state=app.state::<Desktop>();if state.exit_ready.load(Ordering::SeqCst){return;}
            api.prevent_exit();if state.quitting.swap(true,Ordering::SeqCst){return;}
            state.login_cancel.cancel();let host=state.host.clone();let ready=state.exit_ready.clone();let quitting=state.quitting.clone();let status=state.client_error.clone();let app=app.clone();
            tauri::async_runtime::spawn(async move{
                match host.shutdown().await {
                    Ok(())=>{ready.store(true,Ordering::SeqCst);app.exit(0);}
                    Err(error)=>{if let Ok(mut status)=status.lock(){*status=Some(format!("Quit cleanup/save failed: {error}. Application remains open; retry Quit."));}quitting.store(false,Ordering::SeqCst);if let Some(window)=app.get_webview_window("main"){let _=window.show();let _=window.set_focus();}}
                }
            });
        }
        #[cfg(target_os="macos")]
        tauri::RunEvent::Reopen {..}=>{if let Some(window)=app.get_webview_window("main"){let _=window.show();let _=window.set_focus();}}
        _=>{}
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_the_bundled_main_origin_has_command_authority() {
        for url in [
            "tauri://localhost",
            "tauri://localhost/",
            "tauri://localhost/index.html",
        ] {
            assert!(local_origin("main", &tauri::Url::parse(url).unwrap()));
        }
        for (label, url) in [
            ("mascot", "tauri://localhost/"),
            ("main", "https://evil.test/"),
            ("main", "tauri://evil/"),
            ("main", "tauri://localhost/index.html?principal=root"),
            ("main", "tauri://localhost/other.html"),
        ] {
            assert!(!local_origin(label, &tauri::Url::parse(url).unwrap()));
        }
    }
    #[test]
    fn client_selection_restarts_without_permission_persistence_and_rejects_bad_files() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("notes");
        std::fs::create_dir(&path).unwrap();
        settings::save(directory.path(), &path).unwrap();
        assert_eq!(settings::load(directory.path()).unwrap(), Some(path));
        let text = std::fs::read_to_string(directory.path().join("desktop-folder.json")).unwrap();
        assert!(!text.contains("policy"));
        std::fs::write(
            directory.path().join("desktop-folder.json"),
            r#"{"version":99,"path":"/tmp"}"#,
        )
        .unwrap();
        assert!(settings::load(directory.path()).is_err());
    }
}
