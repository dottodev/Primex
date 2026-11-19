//! ZIP/APK/OBB container reader — central directory, ZIP64, local headers,
//! data descriptors, alignment reporting and CRC verification.
//!
//! Read-only in this milestone. The container is never modified in place; a
//! future repack path must rebuild to a new file. All offsets are file
//! offsets. Every validation failure is typed with an offset and expected/
//! actual values.

use pdx_core::error::{PdxError, Result};
use pdx_core::reader::ByteReader;
use serde::Serialize;
use std::collections::HashSet;
use std::io::Read;

const EOCD_SIG: u32 = 0x0605_4b50;
const EOCD64_SIG: u32 = 0x0606_4b50;
const LOCATOR64_SIG: u32 = 0x0706_4b50;
const CENTRAL_SIG: u32 = 0x0201_4b50;
const LOCAL_SIG: u32 = 0x0403_4b50;
const ZIP64_EXTRA_ID: u16 = 0x0001;
const FLAG_DATA_DESCRIPTOR: u16 = 1 << 3;
const FLAG_UTF8_NAME: u16 = 1 << 11;
pub const PAGE_ALIGNMENT: u64 = 16 * 1024;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ZipEntry {
    pub name: String,
    pub method: u16,
    pub flags: u16,
    pub crc32: u32,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub local_header_offset: u64,
    /// Offset of the entry's data relative to `PAGE_ALIGNMENT`. Zero means
    /// the data starts on a 16 KB boundary (required for uncompressed .so).
    pub alignment: u64,
    pub external_attributes: u32,
    pub is_directory: bool,
    pub uses_data_descriptor: bool,
    pub zip64: bool,
}

impl ZipEntry {
    pub fn is_16k_aligned(&self) -> bool {
        self.alignment == 0
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ApkArchive {
    pub entry_count: usize,
    pub entries: Vec<ZipEntry>,
    pub eocd_offset: u64,
    pub central_directory_offset: u64,
    pub central_directory_size: u64,
    pub zip64: bool,
    pub comment: String,
    /// Number of bytes prepended before the first local header (SFX stub).
    pub archive_base: u64,
}

fn u32_at(data: &[u8], offset: usize) -> Option<u32> {
    if offset + 4 <= data.len() {
        Some(u32::from_le_bytes(
            data[offset..offset + 4].try_into().unwrap(),
        ))
    } else {
        None
    }
}

fn find_eocd(data: &[u8]) -> Result<usize> {
    if data.len() < 22 {
        return Err(PdxError::bad_input("file shorter than ZIP EOCD")
            .expected(">= 22 bytes", format!("{} bytes", data.len())));
    }
    let minimum = data.len().saturating_sub(22 + 65_535);
    let mut position = data.len() - 22;
    loop {
        if u32_at(data, position) == Some(EOCD_SIG) {
            let comment_len =
                u16::from_le_bytes(data[position + 20..position + 22].try_into().unwrap()) as usize;
            if position + 22 + comment_len == data.len() {
                return Ok(position);
            }
        }
        if position == minimum {
            break;
        }
        position -= 1;
    }
    Err(
        PdxError::bad_input("ZIP end-of-central-directory record not found")
            .expected(
                "PK\\x05\\x06 signature with matching comment length",
                "none",
            )
            .at(data.len() as u64),
    )
}

/// Parses the ZIP64 extra field, consuming only the placeholder-sized values,
/// in the fixed order ZIP64 writers emit them: uncompressed, compressed,
/// offset.
fn zip64_values(
    extra: &[u8],
    need_uncompressed: bool,
    need_compressed: bool,
    need_offset: bool,
) -> Result<(Option<u64>, Option<u64>, Option<u64>)> {
    let mut reader = ByteReader::new(extra);
    while reader.remaining() >= 4 {
        let id = reader.read_u16()?;
        let size = reader.read_u16()? as usize;
        let field_start = reader.pos();
        if reader.remaining() < size {
            return Err(PdxError::bad_input("truncated ZIP extra field")
                .at(field_start)
                .expected(
                    format!("{size} bytes"),
                    format!("{} bytes", reader.remaining()),
                ));
        }
        let field = reader.read_bytes(size)?;
        if id == ZIP64_EXTRA_ID {
            let mut values = ByteReader::new(field);
            let uncompressed = if need_uncompressed {
                Some(values.read_u64()?)
            } else {
                None
            };
            let compressed = if need_compressed {
                Some(values.read_u64()?)
            } else {
                None
            };
            let offset = if need_offset {
                Some(values.read_u64()?)
            } else {
                None
            };
            return Ok((uncompressed, compressed, offset));
        }
    }
    Err(PdxError::unsupported(
        "ZIP64 placeholder without ZIP64 extra field",
    ))
}

pub fn parse(data: &[u8]) -> Result<ApkArchive> {
    let eocd = find_eocd(data)?;
    let mut reader = ByteReader::new(data);
    reader.seek(eocd as u64)?;
    reader.skip(4)?;
    let _disk = reader.read_u16()?;
    let _cd_disk = reader.read_u16()?;
    let entries_this_disk = reader.read_u16()? as u64;
    let total_entries = reader.read_u16()? as u64;
    let mut cd_size = reader.read_u32()? as u64;
    let mut cd_offset = reader.read_u32()? as u64;
    let comment_len = reader.read_u16()? as usize;
    let comment = String::from_utf8_lossy(reader.read_bytes(comment_len)?).to_string();

    let mut zip64 = false;
    let mut total_entries = total_entries;
    if entries_this_disk == u16::MAX as u64
        || total_entries == u16::MAX as u64
        || cd_size == u32::MAX as u64
        || cd_offset == u32::MAX as u64
    {
        zip64 = true;
        if eocd < 20 {
            return Err(
                PdxError::bad_input("ZIP64 locator does not fit before EOCD").at(eocd as u64),
            );
        }
        let locator = eocd - 20;
        reader.seek(locator as u64)?;
        if reader.read_u32()? != LOCATOR64_SIG {
            return Err(PdxError::bad_input("ZIP64 locator signature mismatch")
                .at(locator as u64)
                .expected(format!("{LOCATOR64_SIG:#010x}"), "other"));
        }
        let zip64_eocd_offset = reader.read_u64()?;
        reader.seek(zip64_eocd_offset)?;
        if reader.read_u32()? != EOCD64_SIG {
            return Err(PdxError::bad_input("ZIP64 EOCD signature mismatch")
                .at(zip64_eocd_offset)
                .expected(format!("{EOCD64_SIG:#010x}"), "other"));
        }
        reader.skip(8)?; // record size
        reader.skip(4)?; // version made by / needed
        reader.skip(8)?; // disk numbers
        let zip64_this_disk = reader.read_u64()?;
        total_entries = reader.read_u64()?;
        cd_size = reader.read_u64()?;
        cd_offset = reader.read_u64()?;
        if zip64_this_disk != total_entries {
            return Err(
                PdxError::unsupported("multi-disk ZIP64 is not supported").expected(
                    format!("{total_entries} entries on one disk"),
                    format!("{zip64_this_disk} on this disk"),
                ),
            );
        }
    } else if entries_this_disk != total_entries {
        return Err(
            PdxError::unsupported("multi-disk ZIP is not supported").expected(
                format!("{total_entries} entries on one disk"),
                format!("{entries_this_disk} on this disk"),
            ),
        );
    }
    if total_entries > 10_000_000 {
        return Err(
            PdxError::bad_input("implausible central directory entry count")
                .expected("<= 10000000", total_entries.to_string()),
        );
    }

    // Account for an SFX/prepended stub: the CD must end exactly at EOCD.
    let cd_end = cd_offset
        .checked_add(cd_size)
        .ok_or_else(|| PdxError::bad_input("central directory size overflow"))?;
    let archive_base = eocd.saturating_sub(cd_end as usize) as u64;
    let actual_cd_offset = archive_base
        .checked_add(cd_offset)
        .ok_or_else(|| PdxError::bad_input("central directory offset overflow"))?;
    if actual_cd_offset as usize > data.len() {
        return Err(
            PdxError::bad_input("central directory offset past end of file")
                .at(actual_cd_offset)
                .expected(format!("<= {}", data.len()), actual_cd_offset.to_string()),
        );
    }

    reader.seek(actual_cd_offset)?;
    let mut entries = Vec::with_capacity(total_entries.min(4096) as usize);
    let mut exact_names: HashSet<String> = HashSet::new();
    let mut folded_names: HashSet<String> = HashSet::new();

    for _ in 0..total_entries {
        let entry_offset = reader.pos();
        if reader.remaining() < 46 {
            return Err(PdxError::bad_input("truncated central directory entry")
                .at(entry_offset)
                .expected("46 bytes", format!("{} bytes", reader.remaining())));
        }
        if reader.read_u32()? != CENTRAL_SIG {
            return Err(PdxError::bad_input("central directory signature mismatch")
                .at(entry_offset)
                .expected(format!("{CENTRAL_SIG:#010x}"), "other"));
        }
        reader.skip(4)?; // version made by / needed
        let flags = reader.read_u16()?;
        let method = reader.read_u16()?;
        reader.skip(4)?; // time / date
        let crc32 = reader.read_u32()?;
        let mut compressed_size = reader.read_u32()? as u64;
        let mut uncompressed_size = reader.read_u32()? as u64;
        let name_len = reader.read_u16()? as usize;
        let extra_len = reader.read_u16()? as usize;
        let comment_len = reader.read_u16()? as usize;
        reader.skip(2)?; // disk start
        reader.skip(2)?; // internal attrs
        let external_attributes = reader.read_u32()?;
        let mut local_header_offset = reader.read_u32()? as u64;

        let name_offset = reader.pos();
        let name_bytes = reader.read_bytes(name_len)?;
        let extra = reader.read_bytes(extra_len)?;
        reader.skip(comment_len)?;

        let name = if flags & FLAG_UTF8_NAME != 0 {
            std::str::from_utf8(name_bytes)
                .map_err(|e| {
                    PdxError::unsupported("entry name is not valid UTF-8")
                        .at(name_offset)
                        .expected("valid UTF-8", format!("invalid at byte {e}"))
                })?
                .to_string()
        } else {
            match std::str::from_utf8(name_bytes) {
                Ok(value) => value.to_string(),
                Err(_) => {
                    return Err(PdxError::unsupported(
                        "non-UTF-8 entry name requires code page table",
                    )
                    .at(name_offset)
                    .expected("UTF-8 flag or ASCII name", "CP437 bytes"));
                }
            }
        };

        if name.starts_with('/') || name.split('/').any(|part| part == "..") {
            return Err(PdxError::bad_input("entry name escapes archive root")
                .at(name_offset)
                .expected("relative path without '..'", name.clone()));
        }

        let needs_zip64 = compressed_size == u32::MAX as u64
            || uncompressed_size == u32::MAX as u64
            || local_header_offset == u32::MAX as u64;
        if needs_zip64 {
            let (u, c, o) = zip64_values(
                extra,
                uncompressed_size == u32::MAX as u64,
                compressed_size == u32::MAX as u64,
                local_header_offset == u32::MAX as u64,
            )?;
            if let Some(value) = u {
                uncompressed_size = value;
            }
            if let Some(value) = c {
                compressed_size = value;
            }
            if let Some(value) = o {
                local_header_offset = value;
            }
        }

        let actual_local_offset = archive_base.saturating_add(local_header_offset);
        if actual_local_offset as usize > data.len() {
            return Err(PdxError::bad_input("local header offset past end of file")
                .at(actual_local_offset)
                .expected(
                    format!("<= {}", data.len()),
                    actual_local_offset.to_string(),
                ));
        }

        if !exact_names.insert(name.clone()) {
            return Err(PdxError::bad_input("duplicate ZIP entry name")
                .at(name_offset)
                .expected("unique names", name.clone()));
        }
        let folded = name.to_ascii_lowercase();
        if !folded_names.insert(folded.clone()) {
            return Err(PdxError::bad_input("case-colliding ZIP entry names")
                .at(name_offset)
                .expected("case-insensitively unique names", folded));
        }

        let is_directory = name.ends_with('/');
        entries.push(ZipEntry {
            name,
            method,
            flags,
            crc32,
            compressed_size,
            uncompressed_size,
            local_header_offset: actual_local_offset,
            alignment: 0,
            external_attributes,
            is_directory,
            uses_data_descriptor: flags & FLAG_DATA_DESCRIPTOR != 0,
            zip64: needs_zip64,
        });
    }

    // Second pass: resolve data offsets, bound-check every entry's compressed
    // region and record alignment. On any failure the whole parse is
    // discarded (S3).
    for entry in entries.iter_mut() {
        let start = data_offset(data, entry)?;
        let end = start
            .checked_add(entry.compressed_size)
            .ok_or_else(|| PdxError::bad_input("entry size overflow").at(start))?;
        if end > data.len() as u64 {
            return Err(PdxError::bad_input("entry data exceeds file size")
                .at(start)
                .expected(
                    format!("{} bytes within archive", entry.compressed_size),
                    format!("{} bytes available to EOF", data.len() as u64 - start),
                ));
        }
        entry.alignment = start % PAGE_ALIGNMENT;
    }

    Ok(ApkArchive {
        entry_count: entries.len(),
        entries,
        eocd_offset: eocd as u64,
        central_directory_offset: actual_cd_offset,
        central_directory_size: cd_size,
        zip64,
        comment,
        archive_base,
    })
}

/// Computes the data offset of an entry by reading its local header.
pub fn data_offset(data: &[u8], entry: &ZipEntry) -> Result<u64> {
    let mut reader = ByteReader::new(data);
    reader.seek(entry.local_header_offset)?;
    if reader.read_u32()? != LOCAL_SIG {
        return Err(PdxError::bad_input("local file header signature mismatch")
            .at(entry.local_header_offset)
            .expected(format!("{LOCAL_SIG:#010x}"), "other"));
    }
    reader.skip(22)?; // fixed fields after signature
    let name_len = reader.read_u16()? as usize;
    let extra_len = reader.read_u16()? as usize;
    Ok(reader.pos() + name_len as u64 + extra_len as u64)
}

/// Verifies one entry. Stored entries are CRC-checked directly; deflated
/// entries are streamed through flate2 and checked on the uncompressed bytes.
pub fn verify_entry(data: &[u8], entry: &ZipEntry) -> Result<()> {
    let start = data_offset(data, entry)? as usize;
    let end = start
        .checked_add(entry.compressed_size as usize)
        .ok_or_else(|| PdxError::bad_input("compressed size overflow"))?;
    if end > data.len() {
        return Err(PdxError::bad_input("entry data truncated")
            .at(start as u64)
            .expected(
                format!("{} bytes", entry.compressed_size),
                format!("{} bytes", data.len() - start),
            ));
    }
    let compressed = &data[start..end];
    let offset = start as u64;
    let (crc, size) = match entry.method {
        0 => {
            let mut hasher = crc32fast::Hasher::new();
            hasher.update(compressed);
            (hasher.finalize(), compressed.len() as u64)
        }
        8 => {
            let mut decoder = flate2::read::DeflateDecoder::new(compressed);
            let mut hasher = crc32fast::Hasher::new();
            let mut buffer = [0u8; 64 * 1024];
            let mut total = 0u64;
            loop {
                let read = decoder.read(&mut buffer).map_err(|e| {
                    PdxError::verification(format!("deflate stream invalid: {e}")).at(offset)
                })?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
                total += read as u64;
            }
            (hasher.finalize(), total)
        }
        other => {
            return Err(PdxError::unsupported("unsupported ZIP compression method")
                .expected("0 (stored) or 8 (deflate)", other.to_string()));
        }
    };
    if crc != entry.crc32 {
        return Err(PdxError::verification("entry CRC-32 mismatch")
            .at(offset)
            .expected(format!("{:08x}", entry.crc32), format!("{crc:08x}")));
    }
    if size != entry.uncompressed_size {
        return Err(PdxError::verification("entry uncompressed size mismatch")
            .at(offset)
            .expected(entry.uncompressed_size.to_string(), size.to_string()));
    }
    Ok(())
}

pub fn verify_all(data: &[u8], archive: &ApkArchive) -> Result<usize> {
    for (index, entry) in archive.entries.iter().enumerate() {
        if entry.is_directory {
            continue;
        }
        verify_entry(data, entry).map_err(|error| {
            PdxError::verification(format!("entry {index} '{}' failed: {error}", entry.name))
                .at(entry.local_header_offset)
        })?;
    }
    Ok(archive.entries.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pdx_core::error::ErrorKind;

    const MINIMAL: &[u8] = include_bytes!("../../../fixtures/minimal.apk");

    #[test]
    fn parses_fixture() {
        let archive = parse(MINIMAL).unwrap();
        assert_eq!(archive.entry_count, 5);
        assert!(archive
            .entries
            .iter()
            .any(|e| e.name == "AndroidManifest.xml"));
        assert!(archive.entries.iter().any(|e| e.name == "classes.dex"));
    }

    #[test]
    fn verify_all_fixture() {
        let archive = parse(MINIMAL).unwrap();
        assert_eq!(verify_all(MINIMAL, &archive).unwrap(), 5);
    }

    #[test]
    fn mutated_crc_is_detected() {
        let archive = parse(MINIMAL).unwrap();
        let mut bytes = MINIMAL.to_vec();
        let entry = archive
            .entries
            .iter()
            .find(|e| e.name == "classes.dex")
            .unwrap();
        let offset = data_offset(&bytes, entry).unwrap() as usize;
        bytes[offset] ^= 0xFF;
        let err = verify_entry(&bytes, entry).unwrap_err();
        assert_eq!(err.kind, ErrorKind::VerificationFailed);
    }

    #[test]
    fn truncated_file_is_typed() {
        let err = parse(&MINIMAL[..MINIMAL.len() - 30]).unwrap_err();
        assert_eq!(err.kind, ErrorKind::BadInput);
    }

    #[test]
    fn bad_magic_is_typed() {
        let mut bytes = MINIMAL.to_vec();
        bytes[0] = 0x00;
        let err = parse(&bytes).unwrap_err();
        assert_eq!(err.kind, ErrorKind::BadInput);
        assert!(err.offset.is_some());
    }

    #[test]
    fn empty_archive_parses() {
        let archive = parse(include_bytes!("../../../fixtures/empty.apk")).unwrap();
        assert_eq!(archive.entry_count, 0);
    }

    #[test]
    fn duplicate_names_are_rejected() {
        let err = parse(include_bytes!("../../../fixtures/duplicate.apk")).unwrap_err();
        assert_eq!(err.kind, ErrorKind::BadInput);
        assert!(err.message.contains("duplicate"));
    }

    #[test]
    fn case_collision_is_rejected() {
        let err = parse(include_bytes!("../../../fixtures/case_collision.apk")).unwrap_err();
        assert_eq!(err.kind, ErrorKind::BadInput);
        assert!(err.message.contains("case"));
    }
}
