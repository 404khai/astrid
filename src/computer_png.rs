//! Bounded, file-free PNG validation in a short-lived helper process.
use crate::cancellation::Cancellation;
use std::{
    io::{self, Cursor, Read, Write},
    path::PathBuf,
    process::Stdio,
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub const MAX_PNG: usize = 1_048_576;
const MAX_RGBA: usize = 16_384_000;
const SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PngError {
    #[error("invalid or unsupported PNG")]
    Invalid,
    #[error("image exceeds observation limits")]
    Limit,
    #[error("image worker unavailable")]
    Worker,
    #[error("image worker cancelled")]
    Cancelled,
    #[error("image worker deadline exceeded")]
    Deadline,
}
pub(crate) struct CanonicalPng {
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
}

fn framing(bytes: &[u8]) -> Result<(u32, u32), PngError> {
    if bytes.len() > MAX_PNG {
        return Err(PngError::Limit);
    }
    if !bytes.starts_with(SIGNATURE) {
        return Err(PngError::Invalid);
    }
    let mut pos = 8usize;
    let mut dimensions = None;
    let mut data = false;
    let mut ended_data = false;
    let mut count = 0;
    while pos < bytes.len() {
        count += 1;
        if count > 1024 || bytes.len() - pos < 12 {
            return Err(PngError::Invalid);
        }
        let len = u32::from_be_bytes(
            bytes[pos..pos + 4]
                .try_into()
                .map_err(|_| PngError::Invalid)?,
        ) as usize;
        let end = pos
            .checked_add(12)
            .and_then(|p| p.checked_add(len))
            .filter(|p| *p <= bytes.len())
            .ok_or(PngError::Invalid)?;
        let kind = &bytes[pos + 4..pos + 8];
        let payload = &bytes[pos + 8..end - 4];
        if crc32fast::hash(&bytes[pos + 4..end - 4])
            != u32::from_be_bytes(
                bytes[end - 4..end]
                    .try_into()
                    .map_err(|_| PngError::Invalid)?,
            )
        {
            return Err(PngError::Invalid);
        }
        match kind {
            b"IHDR" if pos == 8 && len == 13 => {
                let w = u32::from_be_bytes(payload[..4].try_into().map_err(|_| PngError::Invalid)?);
                let h =
                    u32::from_be_bytes(payload[4..8].try_into().map_err(|_| PngError::Invalid)?);
                if w == 0
                    || h == 0
                    || w > 2560
                    || h > 2560
                    || u64::from(w) * u64::from(h) > 4_096_000
                {
                    return Err(PngError::Limit);
                }
                // Restrict to non-interlaced 8-bit RGB/RGBA captures. Fail closed
                // on unsupported captures rather than claiming a universal decoder.
                if payload[8] != 8 || !matches!(payload[9], 2 | 6) || payload[10..] != [0, 0, 0] {
                    return Err(PngError::Invalid);
                }
                dimensions = Some((w, h));
            }
            b"IDAT" if dimensions.is_some() && !ended_data => {
                data = true;
            }
            b"IEND" if data && len == 0 && end == bytes.len() => {
                return dimensions.ok_or(PngError::Invalid);
            }
            b"acTL" | b"fcTL" | b"fdAT" => return Err(PngError::Invalid),
            // Strip ancillary data BEFORE the maintained decoder sees it. This
            // includes compressed profiles/text: never decompress metadata.
            _ if dimensions.is_some() && kind[0].is_ascii_lowercase() => {
                if data {
                    ended_data = true;
                }
            }
            _ => return Err(PngError::Invalid),
        }
        pos = end;
    }
    Err(PngError::Invalid)
}

pub(crate) fn canonicalize(bytes: &[u8]) -> Result<CanonicalPng, PngError> {
    let (width, height) = framing(bytes)?;
    // Framing has already bounded dimensions/chunks. Also bound the maintained
    // decoder's intermediate allocations; caller output is bounded separately.
    let mut stripped = SIGNATURE.to_vec();
    let mut p = 8usize;
    while p < bytes.len() {
        let len =
            u32::from_be_bytes(bytes[p..p + 4].try_into().map_err(|_| PngError::Invalid)?) as usize;
        let end = p + 12 + len;
        if matches!(&bytes[p + 4..p + 8], b"IHDR" | b"IDAT" | b"IEND") {
            stripped.extend_from_slice(&bytes[p..end]);
        }
        p = end;
    }
    let mut decoder = png::Decoder::new_with_limits(
        Cursor::new(stripped),
        png::Limits {
            bytes: 32 * 1024 * 1024,
        },
    );
    decoder.ignore_checksums(false);
    let mut reader = decoder.read_info().map_err(|_| PngError::Invalid)?;
    let size = reader
        .output_buffer_size()
        .filter(|n| *n <= MAX_RGBA)
        .ok_or(PngError::Limit)?;
    let mut pixels = vec![0; size];
    let output = reader
        .next_frame(&mut pixels)
        .map_err(|_| PngError::Invalid)?;
    reader.finish().map_err(|_| PngError::Invalid)?;
    let mut encoded = Vec::new();
    {
        let mut encoder = png::Encoder::new(CappedWriter(&mut encoded), width, height);
        encoder.set_color(output.color_type);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|_| PngError::Invalid)?;
        writer
            .write_image_data(&pixels[..output.buffer_size()])
            .map_err(|_| PngError::Limit)?;
        writer.finish().map_err(|_| PngError::Invalid)?;
    }
    Ok(CanonicalPng {
        width,
        height,
        bytes: encoded,
    })
}
struct CappedWriter<'a>(&'a mut Vec<u8>);
impl Write for CappedWriter<'_> {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        if self.0.len().saturating_add(b.len()) > MAX_PNG {
            return Err(io::Error::other("image limit"));
        }
        self.0.extend_from_slice(b);
        Ok(b.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Internal child entry point, dispatched before CLI/auth/config initialization.
/// The helper consumes only bounded piped PNG bytes and emits canonical PNG bytes.
pub fn worker_stdio() -> Result<(), PngError> {
    let mut input = Vec::new();
    io::stdin()
        .take((MAX_PNG + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|_| PngError::Worker)?;
    let image = canonicalize(&input)?;
    io::stdout()
        .write_all(&image.bytes)
        .map_err(|_| PngError::Worker)
}

pub struct DecodeWorker {
    executable: PathBuf,
    #[cfg(test)]
    inline: bool,
}
impl DecodeWorker {
    /// Host supplies the Astrid executable, never a model-controlled command.
    pub fn new(executable: PathBuf) -> Self {
        Self {
            executable,
            #[cfg(test)]
            inline: false,
        }
    }
    #[cfg(test)]
    pub(crate) fn inline() -> Self {
        Self {
            executable: PathBuf::new(),
            inline: true,
        }
    }
    pub async fn validate(
        &self,
        bytes: &[u8],
        cancel: &Cancellation,
    ) -> Result<(u32, u32), PngError> {
        let image = self.decode(bytes, cancel).await?;
        Ok((image.width, image.height))
    }
    pub(crate) async fn decode(
        &self,
        bytes: &[u8],
        cancel: &Cancellation,
    ) -> Result<CanonicalPng, PngError> {
        framing(bytes)?;
        if cancel.is_cancelled() {
            return Err(PngError::Cancelled);
        }
        #[cfg(test)]
        if self.inline {
            return canonicalize(bytes);
        }
        let mut child = tokio::process::Command::new(&self.executable)
            .arg("__astrid_decode_png")
            .env_clear()
            .current_dir("/")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| PngError::Worker)?;
        let mut stdin = child.stdin.take().ok_or(PngError::Worker)?;
        let stdout = child.stdout.take().ok_or(PngError::Worker)?;
        let result = {
            let io = async {
                let write = async {
                    stdin.write_all(bytes).await?;
                    stdin.shutdown().await?;
                    drop(stdin);
                    Ok::<_, io::Error>(())
                };
                let read = async {
                    let mut result = Vec::new();
                    stdout
                        .take((MAX_PNG + 1) as u64)
                        .read_to_end(&mut result)
                        .await?;
                    Ok::<_, io::Error>(result)
                };
                let (_, output, status) =
                    tokio::try_join!(write, read, child.wait()).map_err(|_| PngError::Worker)?;
                if !status.success() {
                    return Err(PngError::Invalid);
                }
                let (width, height) = framing(&output)?;
                Ok(CanonicalPng {
                    width,
                    height,
                    bytes: output,
                })
            };
            tokio::select! {biased;
                _=cancel.cancelled()=>Err(PngError::Cancelled),
                _=tokio::time::sleep(Duration::from_secs(2))=>Err(PngError::Deadline),
                value=io=>value,
            }
        };
        if result.is_err() {
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    pub(crate) fn fixture() -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut e = png::Encoder::new(&mut bytes, 2, 2);
            e.set_color(png::ColorType::Rgb);
            e.set_depth(png::BitDepth::Eight);
            let mut w = e.write_header().unwrap();
            w.write_image_data(&[40; 12]).unwrap();
        }
        bytes
    }
    #[test]
    fn bounded_rgb_canonicalization_crc_and_trailing_rejection() {
        let bytes = fixture();
        let c = canonicalize(&bytes).unwrap();
        assert_eq!((c.width, c.height), (2, 2));
        assert!(canonicalize(&c.bytes).is_ok());
        for bad in [
            bytes[..bytes.len() - 1].to_vec(),
            [bytes.as_slice(), b"suffix"].concat(),
            vec![0; MAX_PNG + 1],
        ] {
            assert!(canonicalize(&bad).is_err());
        }
        let mut bad = bytes;
        bad[29] ^= 1;
        assert!(canonicalize(&bad).is_err());
    }
    fn chunk(kind: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut b = (payload.len() as u32).to_be_bytes().to_vec();
        b.extend_from_slice(kind);
        b.extend_from_slice(payload);
        let crc = crc32fast::hash(&b[4..]);
        b.extend_from_slice(&crc.to_be_bytes());
        b
    }
    #[test]
    fn strips_text_profiles_before_decoding_and_rejects_animation_geometry_and_bombs() {
        let base = fixture();
        // Deliberately invalid compressed profile: safe because never decompressed.
        let bytes = [
            &base[..33],
            &chunk(b"zTXt", b"private-profile\0\0invalid-zlib"),
            &base[33..],
        ]
        .concat();
        let clean = canonicalize(&bytes).unwrap();
        assert!(!clean.bytes.windows(4).any(|b| b == b"zTXt"));
        assert!(!clean.bytes.windows(7).any(|b| b == b"private"));
        for kind in [b"acTL", b"fcTL", b"fdAT", b"ABCD"] {
            assert!(
                canonicalize(&[&base[..33], &chunk(kind, &[0; 8]), &base[33..]].concat()).is_err()
            );
        }
        let mut header = base[16..29].to_vec();
        header[..4].copy_from_slice(&2561u32.to_be_bytes());
        let oversized = [&base[..8], chunk(b"IHDR", &header).as_slice(), &base[33..]].concat();
        assert!(matches!(canonicalize(&oversized), Err(PngError::Limit)));
        // Valid framing with invalid compressed pixel stream never mints a receipt.
        let bomb = [
            &base[..33],
            chunk(b"IDAT", &[0xff; 1024]).as_slice(),
            chunk(b"IEND", &[]).as_slice(),
        ]
        .concat();
        assert!(canonicalize(&bomb).is_err());
        let many = (0..1024)
            .flat_map(|_| chunk(b"tEXt", b"x"))
            .collect::<Vec<_>>();
        assert!(canonicalize(&[&base[..33], many.as_slice(), &base[33..]].concat()).is_err());
    }
}
