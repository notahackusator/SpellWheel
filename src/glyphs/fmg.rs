/// Ported from SoulsFormats

use anyhow::{bail, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FmgVersion {
    DemonsSouls = 0,
    DarkSouls1 = 1,
    /// Also covers Bloodborne, and — per the C# source's use of this same enum
    /// value for later "modern" FMGs — Sekiro/ER. Worth confirming the byte
    /// you actually see at this offset is 2 when you test against a real file;
    /// I'm inferring the ER case rather than having verified it directly.
    DarkSouls3 = 2,
}

impl FmgVersion {
    fn from_u8(b: u8) -> Result<Self> {
        Ok(match b {
            0 => FmgVersion::DemonsSouls,
            1 => FmgVersion::DarkSouls1,
            2 => FmgVersion::DarkSouls3,
            other => bail!("unknown FMG version byte: {other}"),
        })
    }
}

#[derive(Debug, Clone)]
pub struct FmgEntry {
    pub id: i32,
    pub text: Option<String>,
}

#[derive(Debug)]
pub struct Fmg {
    pub entries: Vec<FmgEntry>,
    pub version: FmgVersion,
    pub big_endian: bool,
    pub unicode: bool,
    pub md5: bool,
}

/// Minimal cursor mirroring the subset of `BinaryReaderEx` behavior `FMG::Read`
/// relies on: absolute-offset peeks, a position stack (StepIn/StepOut), and
/// endian/width-aware int reads.
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    big_endian: bool,
    /// Mirrors `VarintLong`: true means "varint" fields are 8 bytes (DS3/ER), else 4.
    wide: bool,
    stack: Vec<usize>,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0, big_endian: false, wide: false, stack: Vec::new() }
    }

    fn peek_u8(&self, at: usize) -> Result<u8> {
        self.data.get(at).copied().ok_or_else(|| anyhow::anyhow!("read past end of buffer"))
    }

    fn u8(&mut self) -> Result<u8> {
        let b = self.peek_u8(self.pos)?;
        self.pos += 1;
        Ok(b)
    }

    fn assert_u8(&mut self, expected: u8) -> Result<()> {
        let b = self.u8()?;
        if b != expected {
            bail!("expected byte {expected:#x} at offset {}, got {b:#x}", self.pos - 1);
        }
        Ok(())
    }

    fn bool8(&mut self) -> Result<bool> {
        Ok(self.u8()? != 0)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).ok_or_else(|| anyhow::anyhow!("overflow"))?;
        let slice = self.data.get(self.pos..end).ok_or_else(|| anyhow::anyhow!("read past end of buffer"))?;
        self.pos = end;
        Ok(slice)
    }

    fn i32(&mut self) -> Result<i32> {
        let bytes = self.take(4)?;
        Ok(if self.big_endian {
            i32::from_be_bytes(bytes.try_into().unwrap())
        } else {
            i32::from_le_bytes(bytes.try_into().unwrap())
        })
    }

    fn assert_i32(&mut self, expected: i32) -> Result<()> {
        let v = self.i32()?;
        if v != expected {
            bail!("expected i32 {expected} at offset {}, got {v}", self.pos - 4);
        }
        Ok(())
    }

    /// Mirrors `ReadVarint`: 8-byte int when `wide` (DS3/ER), else 4-byte,
    /// both sign-extended to i64.
    fn varint(&mut self) -> Result<i64> {
        if self.wide {
            let bytes = self.take(8)?;
            Ok(if self.big_endian {
                i64::from_be_bytes(bytes.try_into().unwrap())
            } else {
                i64::from_le_bytes(bytes.try_into().unwrap())
            })
        } else {
            Ok(self.i32()? as i64)
        }
    }

    fn assert_varint(&mut self, expected: i64) -> Result<()> {
        let v = self.varint()?;
        if v != expected {
            bail!("expected varint {expected} at offset {}, got {v}", self.pos);
        }
        Ok(())
    }

    fn step_in(&mut self, at: i64) -> Result<()> {
        self.stack.push(self.pos);
        self.pos = usize::try_from(at).map_err(|_| anyhow::anyhow!("negative offset"))?;
        Ok(())
    }

    fn step_out(&mut self) -> Result<()> {
        self.pos = self.stack.pop().ok_or_else(|| anyhow::anyhow!("step_out without matching step_in"))?;
        Ok(())
    }

    /// Null-terminated UTF-16 string at an absolute offset, cursor untouched.
    fn get_utf16(&self, offset: i64) -> Result<String> {
        let mut i = usize::try_from(offset).map_err(|_| anyhow::anyhow!("negative offset"))?;
        let mut units = Vec::new();
        loop {
            let bytes = self.data.get(i..i + 2)
                .ok_or_else(|| anyhow::anyhow!("UTF-16 read past end of buffer"))?;
            let unit = if self.big_endian {
                u16::from_be_bytes(bytes.try_into().unwrap())
            } else {
                u16::from_le_bytes(bytes.try_into().unwrap())
            };
            i += 2;
            if unit == 0 {
                break;
            }
            units.push(unit);
        }
        Ok(char::decode_utf16(units)
            .map(|r| r.unwrap_or(char::REPLACEMENT_CHARACTER))
            .collect())
    }

    /// Null-terminated Shift-JIS string. Only hit when `Unicode == false`
    /// (DeS-era files) — ER always sets Unicode, so this branch is likely dead
    /// code for your use case. Stubbed to avoid pulling in `encoding_rs` for
    /// a path you probably never take; fill in if you end up needing it.
    fn get_shift_jis(&self, _offset: i64) -> Result<String> {
        bail!("Shift-JIS FMG entries not supported (unexpected for ER data)")
    }
}

impl Fmg {
    pub fn read(data: &[u8]) -> Result<Fmg> {
        let mut r = Reader::new(data);

        let md5 = r.peek_u8(0)? != 0;
        if md5 {
            r.pos += 16; // skip MD5 hash
        }

        r.assert_u8(0)?;
        let big_endian = r.bool8()?;
        r.big_endian = big_endian;
        let version = FmgVersion::from_u8(r.u8()?)?;
        r.assert_u8(0)?;

        let wide = version == FmgVersion::DarkSouls3;
        r.wide = wide;

        let _file_size = r.i32()?;
        let unicode = r.bool8()?;
        r.assert_u8(if version == FmgVersion::DemonsSouls { 0xFF } else { 0x00 })?;
        r.assert_u8(0)?;
        r.assert_u8(0)?;
        let group_count = r.i32()?;
        let _string_count = r.i32()?;

        if wide {
            r.assert_i32(0xFF)?;
        }

        let mut string_offsets_offset = r.varint()?;
        if md5 {
            string_offsets_offset += 16;
        }
        r.assert_varint(0)?;

        let mut entries = Vec::new();
        for _ in 0..group_count {
            let offset_index = r.i32()?;
            let first_id = r.i32()?;
            let last_id = r.i32()?;

            if wide {
                r.assert_i32(0)?;
            }

            let stride: i64 = if wide { 8 } else { 4 };
            r.step_in(string_offsets_offset + offset_index as i64 * stride)?;

            for j in 0..=(last_id - first_id) {
                let mut string_offset = r.varint()?;
                if md5 {
                    string_offset += 16;
                }

                let text = if string_offset > 0 {
                    Some(if unicode {
                        r.get_utf16(string_offset)?
                    } else {
                        r.get_shift_jis(string_offset)?
                    })
                } else {
                    None
                };

                entries.push(FmgEntry { id: first_id + j, text });
            }

            r.step_out()?;
        }

        Ok(Fmg { entries, version, big_endian, unicode, md5 })
    }
}