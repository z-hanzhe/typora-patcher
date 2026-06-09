// Minimal Electron Fuses patcher (V1 only).
//
// Layout in the binary:
//   [sentinel: 32 bytes ASCII] [version: 1 byte] [fuse_count: 1 byte] [fuses: fuse_count bytes]
// Each fuse byte:
//   '0' (0x30) INHERIT, '1' (0x31) ENABLE, '2' (0x32) DISABLE, 'r' (0x72) REMOVED
//
// We flip OnlyLoadAppFromAsar (index 5 in V1) to DISABLE.

use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

const SENTINEL: &[u8] = b"dL7pKGdnNz796PbbjQWNKmHXBZaB9tsX";
const FUSE_DISABLE: u8 = b'0' + 2;

pub fn disable_only_load_app_from_asar<P: AsRef<Path>>(exe: P) -> std::io::Result<bool> {
    let exe = exe.as_ref();
    let mut f = OpenOptions::new().read(true).write(true).open(exe)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;

    let idx = match find_subsequence(&buf, SENTINEL) {
        Some(i) => i,
        None => return Ok(false),
    };

    let version_pos = idx + SENTINEL.len();
    if buf.len() < version_pos + 2 {
        return Ok(false);
    }
    let version = buf[version_pos];
    if version != 1 {
        // Only V1 supported.
        return Ok(false);
    }
    let count = buf[version_pos + 1] as usize;
    let fuses_pos = version_pos + 2;
    if buf.len() < fuses_pos + count {
        return Ok(false);
    }

    // OnlyLoadAppFromAsar = index 5
    let target_index = 5usize;
    if target_index >= count {
        return Ok(false);
    }

    let write_pos = (fuses_pos + target_index) as u64;
    f.seek(SeekFrom::Start(write_pos))?;
    f.write_all(&[FUSE_DISABLE])?;
    f.flush()?;
    Ok(true)
}

fn find_subsequence(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}
