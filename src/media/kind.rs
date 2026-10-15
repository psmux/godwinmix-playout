//! What a file is, from its first bytes rather than its name. A clip called
//! `.mov` that is really an MP4, or a `.jpg` that is a PNG, still plays.

use super::Res;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    Mp4,
    Matroska,
    Picture,
    Audio,
    Unknown,
}

pub fn sniff(path: &Path) -> Res<Container> {
    let mut head = [0u8; 16];
    let mut file = std::fs::File::open(path).map_err(|e| open_error(path, &e))?;
    let n = file.read(&mut head).map_err(|e| format!("could not read {}: {e}", path.display()))?;
    Ok(classify(&head[..n]))
}

pub fn open_error(path: &Path, e: &std::io::Error) -> String {
    match e.kind() {
        std::io::ErrorKind::NotFound => format!("{} is not there. Check the path, or remove it from the list", path.display()),
        std::io::ErrorKind::PermissionDenied => format!("{} cannot be read by the mixer's user", path.display()),
        _ => format!("could not open {}: {e}", path.display()),
    }
}

pub fn classify(head: &[u8]) -> Container {
    let at = |i: usize, s: &[u8]| head.len() >= i + s.len() && &head[i..i + s.len()] == s;
    if head.len() >= 8 && [&b"ftyp"[..], b"moov", b"mdat", b"free", b"wide", b"skip"].iter().any(|b| at(4, b)) {
        return Container::Mp4;
    }
    if at(0, &[0x1a, 0x45, 0xdf, 0xa3]) {
        return Container::Matroska;
    }
    if at(0, &[0x89, b'P', b'N', b'G']) || at(0, &[0xff, 0xd8, 0xff]) || at(0, b"GIF8") || at(0, b"BM")
        || (at(0, b"RIFF") && at(8, b"WEBP")) || at(0, b"II*\0") || at(0, b"MM\0*")
    {
        return Container::Picture;
    }
    if at(0, b"ID3") || at(0, b"fLaC") || at(0, b"OggS") || (at(0, b"RIFF") && at(8, b"WAVE"))
        || (at(0, b"FORM") && (at(8, b"AIFF") || at(8, b"AIFC")))
        || (head.len() >= 2 && head[0] == 0xff && head[1] & 0xe0 == 0xe0)
    {
        return Container::Audio;
    }
    Container::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_bytes_decide() {
        assert_eq!(classify(b"\0\0\0\x18ftypisom"), Container::Mp4);
        assert_eq!(classify(&[0x1a, 0x45, 0xdf, 0xa3, 1]), Container::Matroska);
        assert_eq!(classify(b"\x89PNG\r\n\x1a\n"), Container::Picture);
        assert_eq!(classify(b"RIFF\0\0\0\0WEBPVP8 "), Container::Picture);
        assert_eq!(classify(b"RIFF\0\0\0\0WAVEfmt "), Container::Audio);
        assert_eq!(classify(b"ID3\x04"), Container::Audio);
        assert_eq!(classify(&[0xff, 0xfb, 0x90]), Container::Audio);
        assert_eq!(classify(b"hello"), Container::Unknown);
    }

    #[test]
    fn a_missing_file_says_so_in_words() {
        let e = sniff(Path::new("definitely/not/here.mp4")).unwrap_err();
        assert!(e.contains("is not there"), "{e}");
    }
}
