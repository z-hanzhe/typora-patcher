use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

use serde_json::Value;

pub struct Asar {
    file: File,
    header: Value,
    data_start: u64,
}

impl Asar {
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let mut file = File::open(path)?;

        let mut size_buf = [0u8; 8];
        file.read_exact(&mut size_buf)?;
        // bytes 0..4: u32 = 4 (outer pickle payload size)
        // bytes 4..8: u32 = inner pickle total size in file (S)
        let s = u32::from_le_bytes(size_buf[4..8].try_into().unwrap()) as usize;

        let mut header_buf = vec![0u8; s];
        file.read_exact(&mut header_buf)?;
        // header_buf[0..4]: u32 = inner payload size (string-with-padding)
        // header_buf[4..8]: u32 = JSON length
        if header_buf.len() < 8 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "asar header too small",
            ));
        }
        let json_len = u32::from_le_bytes(header_buf[4..8].try_into().unwrap()) as usize;
        if 8 + json_len > header_buf.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "asar json length exceeds header buffer",
            ));
        }
        let json_bytes = &header_buf[8..8 + json_len];
        let header: Value = serde_json::from_slice(json_bytes).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, format!("asar json: {e}"))
        })?;

        let data_start = 8u64 + s as u64;
        Ok(Self { file, header, data_start })
    }

    pub fn extract_all<P: AsRef<Path>>(&mut self, dst: P) -> std::io::Result<()> {
        let dst = dst.as_ref();
        fs::create_dir_all(dst)?;
        let header = self.header.clone();
        self.extract_node(&header, dst)
    }

    fn extract_node(&mut self, node: &Value, dst: &Path) -> std::io::Result<()> {
        let files = match node.get("files").and_then(|v| v.as_object()) {
            Some(o) => o,
            None => return Ok(()),
        };

        for (name, child) in files {
            let target = dst.join(name);
            if child.get("files").is_some() {
                fs::create_dir_all(&target)?;
                self.extract_node(child, &target)?;
            } else if let Some(link) = child.get("link").and_then(|v| v.as_str()) {
                // Symlink in archive: write a small text marker. Typora's asar contains
                // none on Windows, but we keep this branch defensive.
                let _ = link;
                fs::write(&target, b"")?;
            } else if let (Some(offset_str), Some(size_v)) =
                (child.get("offset").and_then(|v| v.as_str()), child.get("size"))
            {
                let offset: u64 = offset_str.parse().map_err(|_| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "bad offset")
                })?;
                let size: u64 = size_v.as_u64().ok_or_else(|| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, "bad size")
                })?;

                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent)?;
                }
                self.file.seek(SeekFrom::Start(self.data_start + offset))?;
                let mut remaining = size;
                let mut out = File::create(&target)?;
                let mut buf = vec![0u8; 64 * 1024];
                while remaining > 0 {
                    let want = remaining.min(buf.len() as u64) as usize;
                    self.file.read_exact(&mut buf[..want])?;
                    out.write_all(&buf[..want])?;
                    remaining -= want as u64;
                }
            }
        }
        Ok(())
    }
}

pub fn extract_all<P: AsRef<Path>, Q: AsRef<Path>>(asar: P, dst: Q) -> std::io::Result<()> {
    let mut a = Asar::open(asar)?;
    a.extract_all(dst)
}

pub fn copy_dir_all<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> std::io::Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to: PathBuf = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&from, &to)?;
        } else if ty.is_file() {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}
