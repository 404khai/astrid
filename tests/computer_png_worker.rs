use astrid::{
    cancellation::Cancellation,
    computer_png::{DecodeWorker, PngError},
};
use std::{os::unix::fs::PermissionsExt, path::PathBuf, time::Duration};

fn fixture() -> Vec<u8> {
    let mut b = Vec::new();
    {
        let mut e = png::Encoder::new(&mut b, 2, 2);
        e.set_color(png::ColorType::Rgb);
        e.set_depth(png::BitDepth::Eight);
        let mut w = e.write_header().unwrap();
        w.write_image_data(&[10; 12]).unwrap();
    }
    b
}
#[tokio::test]
async fn actual_astrid_helper_decodes_only_piped_fixture_without_auth_or_image_files() {
    let worker = DecodeWorker::new(PathBuf::from(env!("CARGO_BIN_EXE_astrid")));
    assert_eq!(
        worker
            .validate(&fixture(), &Cancellation::default())
            .await
            .unwrap(),
        (2, 2)
    );
    let mut bad = fixture();
    bad[29] ^= 1;
    assert_eq!(
        worker.validate(&bad, &Cancellation::default()).await,
        Err(PngError::Invalid)
    );
    let cancel = Cancellation::default();
    cancel.cancel();
    assert_eq!(
        DecodeWorker::new("/no-such-worker".into())
            .validate(&fixture(), &cancel)
            .await,
        Err(PngError::Cancelled)
    );
}

fn stalled_worker(root: &std::path::Path) -> (DecodeWorker, PathBuf) {
    let script = root.join("decode-fixture.sh");
    let pid = root.join("fixture.pid");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\nprintf '%s' \"$$\" > '{}'\nexec /bin/sleep 60\n",
            pid.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o700)).unwrap();
    (DecodeWorker::new(script), pid)
}
fn assert_reaped(pid: &std::path::Path) {
    let pid: i32 = std::fs::read_to_string(pid).unwrap().parse().unwrap();
    assert_eq!(unsafe { libc::kill(pid, 0) }, -1);
    assert_eq!(
        std::io::Error::last_os_error().raw_os_error(),
        Some(libc::ESRCH)
    );
}
#[tokio::test]
async fn stalled_worker_deadline_kills_and_reaps_the_owned_child() {
    let root = tempfile::tempdir().unwrap();
    let (worker, pid) = stalled_worker(root.path());
    assert_eq!(
        worker.validate(&fixture(), &Cancellation::default()).await,
        Err(PngError::Deadline)
    );
    assert_reaped(&pid);
}
#[tokio::test]
async fn cancelling_worker_kills_and_reaps_without_detached_decode_work() {
    let root = tempfile::tempdir().unwrap();
    let (worker, pid) = stalled_worker(root.path());
    let cancel = Cancellation::default();
    let bytes = fixture();
    let decode = worker.validate(&bytes, &cancel);
    let cancellation = async {
        tokio::time::timeout(Duration::from_secs(1), async {
            while !pid.exists() {
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .unwrap();
        cancel.cancel();
    };
    let (result, ()) = tokio::join!(decode, cancellation);
    assert_eq!(result, Err(PngError::Cancelled));
    assert_reaped(&pid);
}
