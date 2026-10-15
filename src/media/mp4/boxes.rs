//! ISO BMFF boxes, read from a byte slice. Only what playing a file needs.

/// A cursor over big endian fields, which answers zero past the end rather
/// than panicking: a truncated box gives a wrong number, and the caller's
/// checks on that number say what is wrong in words.
pub struct Be<'a> {
    pub data: &'a [u8],
    pub at: usize,
}

impl<'a> Be<'a> {
    pub fn new(data: &'a [u8]) -> Be<'a> {
        Be { data, at: 0 }
    }
    pub fn skip(&mut self, n: usize) {
        self.at += n;
    }
    pub fn bytes(&mut self, n: usize) -> &'a [u8] {
        let start = self.at.min(self.data.len());
        let end = (self.at + n).min(self.data.len());
        self.at += n;
        &self.data[start..end]
    }
    pub fn u8(&mut self) -> u8 {
        self.bytes(1).first().copied().unwrap_or(0)
    }
    pub fn u16(&mut self) -> u16 {
        self.bytes(2).iter().fold(0u16, |v, b| (v << 8) | *b as u16)
    }
    pub fn u32(&mut self) -> u32 {
        self.bytes(4).iter().fold(0u32, |v, b| (v << 8) | *b as u32)
    }
    pub fn u64(&mut self) -> u64 {
        self.bytes(8).iter().fold(0u64, |v, b| (v << 8) | *b as u64)
    }
    pub fn left(&self) -> usize {
        self.data.len().saturating_sub(self.at)
    }
}

/// One box: its four character type and its body.
#[derive(Clone, Copy)]
pub struct Bx<'a> {
    pub kind: [u8; 4],
    pub body: &'a [u8],
}

/// The boxes laid end to end in `data`.
pub fn children(data: &[u8]) -> Vec<Bx<'_>> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at + 8 <= data.len() {
        let mut r = Be::new(&data[at..]);
        let size32 = r.u32() as u64;
        let kind: [u8; 4] = r.bytes(4).try_into().unwrap_or([0; 4]);
        let (header, size) = match size32 {
            1 => (16usize, r.u64()),
            0 => (8, (data.len() - at) as u64),
            n => (8, n),
        };
        if size < header as u64 || at as u64 + size > data.len() as u64 {
            break;
        }
        out.push(Bx { kind, body: &data[at + header..at + size as usize] });
        at += size as usize;
    }
    out
}

/// The first child of type `kind`.
pub fn child<'a>(data: &'a [u8], kind: &[u8; 4]) -> Option<&'a [u8]> {
    children(data).into_iter().find(|b| &b.kind == kind).map(|b| b.body)
}

/// Down a path of types: `path(moov, &[b"trak", b"mdia"])`.
pub fn path<'a>(data: &'a [u8], kinds: &[&[u8; 4]]) -> Option<&'a [u8]> {
    kinds.iter().try_fold(data, |d, k| child(d, k))
}

/// A full box's version, and the body after its version and flags.
pub fn full(body: &[u8]) -> (u8, &[u8]) {
    if body.len() < 4 {
        return (0, &[]);
    }
    (body[0], &body[4..])
}

/// An MPEG-4 descriptor length: up to four bytes of seven bits each.
pub fn descriptor_len(r: &mut Be) -> usize {
    let mut len = 0usize;
    for _ in 0..4 {
        let b = r.u8();
        len = (len << 7) | (b & 0x7f) as usize;
        if b & 0x80 == 0 {
            break;
        }
    }
    len
}

/// From an `esds` body: the object type and the decoder specific info.
pub fn esds(body: &[u8]) -> Option<(u8, Vec<u8>)> {
    let (_, body) = full(body);
    let mut r = Be::new(body);
    if r.u8() != 3 {
        return None;
    }
    descriptor_len(&mut r);
    r.skip(2);
    let flags = r.u8();
    if flags & 0x80 != 0 {
        r.skip(2);
    }
    if flags & 0x40 != 0 {
        let n = r.u8() as usize;
        r.skip(n);
    }
    if flags & 0x20 != 0 {
        r.skip(2);
    }
    if r.u8() != 4 {
        return None;
    }
    descriptor_len(&mut r);
    let object_type = r.u8();
    r.skip(12);
    if r.left() == 0 || r.u8() != 5 {
        return Some((object_type, Vec::new()));
    }
    let n = descriptor_len(&mut r);
    Some((object_type, r.bytes(n).to_vec()))
}

#[cfg(test)]
#[path = "boxes_tests.rs"]
mod tests;
