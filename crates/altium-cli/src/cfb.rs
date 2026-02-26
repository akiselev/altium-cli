use std::collections::BTreeSet;
use std::io::{Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use altium_format_types::constants::parsing::{
    BLOCK_FLAG_BINARY, BLOCK_FLAG_SHIFT, BLOCK_SIZE_MASK,
};
use clap::{Args, Subcommand};

#[derive(Subcommand)]
pub enum CfbSubcommand {
    /// List all streams and storages in a CFB file
    Ls(LsArgs),
    /// Hex+ASCII dump of a stream
    Dump(DumpArgs),
    /// Block-level inspection of a stream
    Blocks(BlocksArgs),
    /// Compare two CFB files stream by stream
    Diff(DiffArgs),
    /// Output raw stream bytes to stdout (for piping)
    Cat(CatArgs),
}

#[derive(Args)]
pub struct LsArgs {
    /// Path to the CFB file
    file: PathBuf,
    /// Flat output: one entry per line, tab-separated (path, kind, size)
    #[arg(long)]
    flat: bool,
}

#[derive(Args)]
pub struct DumpArgs {
    /// Path to the CFB file
    file: PathBuf,
    /// Stream path within the CFB (e.g. /FileHeader)
    stream: String,
    /// Annotate block boundaries and decode text blocks
    #[arg(long)]
    blocks: bool,
    /// Start offset in bytes
    #[arg(long)]
    offset: Option<usize>,
    /// Maximum number of bytes to display
    #[arg(long)]
    limit: Option<usize>,
}

#[derive(Args)]
pub struct BlocksArgs {
    /// Path to the CFB file
    file: PathBuf,
    /// Stream path within the CFB (e.g. /Component_1/Data)
    stream: String,
    /// Show full detail for a single block (hex dump + decoded text)
    #[arg(long)]
    block: Option<usize>,
}

#[derive(Args)]
pub struct DiffArgs {
    /// First file to compare
    file1: PathBuf,
    /// Second file to compare
    file2: PathBuf,
    /// Show block-level diffs (parse 4-byte block headers)
    #[arg(long)]
    blocks: bool,
    /// Show identical streams as OK instead of skipping them
    #[arg(long, short)]
    verbose: bool,
    /// Filter comparison to a single stream
    #[arg(long)]
    stream: Option<String>,
    /// Use semantic diff (order-agnostic params, embedded object decompression)
    #[arg(long)]
    semantic: bool,
    /// Case-insensitive key comparison (semantic mode only)
    #[arg(long)]
    case_insensitive_keys: bool,
}

#[derive(Args)]
pub struct CatArgs {
    /// Path to the CFB file
    file: PathBuf,
    /// Stream path within the CFB
    stream: String,
}

// ── Block parsing ────────────────────────────────────────────────────────────

struct ParsedBlock<'a> {
    index: usize,
    header_offset: usize,
    flags: u8,
    size: u32,
    payload: &'a [u8],
}

impl ParsedBlock<'_> {
    fn is_binary(&self) -> bool {
        self.flags & BLOCK_FLAG_BINARY != 0
    }

    fn format_label(&self) -> &'static str {
        if self.is_binary() { "binary" } else { "text" }
    }
}

fn parse_blocks(data: &[u8]) -> Vec<ParsedBlock<'_>> {
    let mut blocks = Vec::new();
    let mut pos = 0;
    let mut index = 0;

    while pos + 4 <= data.len() {
        let raw = u32::from_le_bytes([data[pos], data[pos + 1], data[pos + 2], data[pos + 3]]);
        let flags = (raw >> BLOCK_FLAG_SHIFT) as u8;
        let size = raw & BLOCK_SIZE_MASK;

        let payload_start = pos + 4;
        let payload_end = payload_start + size as usize;
        if payload_end > data.len() {
            break;
        }

        blocks.push(ParsedBlock {
            index,
            header_offset: pos,
            flags,
            size,
            payload: &data[payload_start..payload_end],
        });

        pos = payload_end;
        index += 1;
    }

    blocks
}

fn decode_text_block(payload: &[u8]) -> String {
    let (decoded, _) = encoding_rs::WINDOWS_1252.decode_without_bom_handling(payload);
    // Strip trailing NUL if present (Altium text blocks are NUL-terminated)
    let s = decoded.into_owned();
    s.trim_end_matches('\0').to_owned()
}

// ── Hex formatting ───────────────────────────────────────────────────────────

fn format_hex_line(offset: usize, bytes: &[u8]) -> String {
    let mut hex_part = String::with_capacity(48);
    let mut ascii_part = String::with_capacity(16);

    for (i, &b) in bytes.iter().enumerate() {
        if i > 0 && i % 8 == 0 {
            hex_part.push(' ');
        }
        hex_part.push_str(&format!("{b:02x} "));
        ascii_part.push(if b.is_ascii_graphic() || b == b' ' {
            b as char
        } else {
            '.'
        });
    }

    // Pad hex part to align ASCII column (16 bytes = 48 hex chars + 1 mid-gap)
    let expected_hex_len = 49; // 16*3 + 1 gap
    while hex_part.len() < expected_hex_len {
        hex_part.push(' ');
    }

    format!("{offset:08x}: {hex_part} {ascii_part}")
}

fn print_hex_dump(data: &[u8], start: usize, limit: Option<usize>) {
    let end = match limit {
        Some(n) => (start + n).min(data.len()),
        None => data.len(),
    };
    let slice = &data[start..end];

    for (i, chunk) in slice.chunks(16).enumerate() {
        println!("{}", format_hex_line(start + i * 16, chunk));
    }
}

// ── CFB helpers ──────────────────────────────────────────────────────────────

fn open_cfb(path: &Path) -> anyhow::Result<cfb::CompoundFile<std::io::Cursor<Vec<u8>>>> {
    let bytes = std::fs::read(path)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", path.display()))?;
    cfb::CompoundFile::open(std::io::Cursor::new(bytes))
        .map_err(|e| anyhow::anyhow!("failed to open CFB {}: {e}", path.display()))
}

fn read_stream<F: std::io::Read + std::io::Seek>(
    comp: &mut cfb::CompoundFile<F>,
    path: &Path,
) -> anyhow::Result<Vec<u8>> {
    let mut stream = comp.open_stream(path)?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    Ok(buf)
}

fn collect_entries<F: std::io::Read + std::io::Seek>(
    comp: &mut cfb::CompoundFile<F>,
) -> BTreeSet<String> {
    comp.walk()
        .filter(|e| !e.is_root())
        .map(|e| e.path().to_string_lossy().into_owned())
        .collect()
}

// ── Entry point ──────────────────────────────────────────────────────────────

pub fn run(sub: CfbSubcommand) -> anyhow::Result<ExitCode> {
    match sub {
        CfbSubcommand::Ls(args) => {
            cmd_ls(&args)?;
            Ok(ExitCode::SUCCESS)
        }
        CfbSubcommand::Dump(args) => {
            cmd_dump(&args)?;
            Ok(ExitCode::SUCCESS)
        }
        CfbSubcommand::Blocks(args) => {
            cmd_blocks(&args)?;
            Ok(ExitCode::SUCCESS)
        }
        CfbSubcommand::Diff(args) => {
            let identical = if args.semantic {
                cmd_diff_semantic(&args)?
            } else {
                cmd_diff(&args)?
            };
            Ok(if identical {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            })
        }
        CfbSubcommand::Cat(args) => {
            cmd_cat(&args)?;
            Ok(ExitCode::SUCCESS)
        }
    }
}

// ── Subcommand implementations ───────────────────────────────────────────────

fn cmd_ls(args: &LsArgs) -> anyhow::Result<()> {
    let mut comp = open_cfb(&args.file)?;

    if args.flat {
        for entry in comp.walk() {
            if entry.is_root() {
                continue;
            }
            let path = entry.path().to_string_lossy().into_owned();
            let kind = if entry.is_storage() {
                "storage"
            } else {
                "stream"
            };
            let size = if entry.is_stream() { entry.len() } else { 0 };
            println!("{path}\t{kind}\t{size}");
        }
    } else {
        print_tree(&mut comp)?;
    }

    Ok(())
}

fn print_tree<F: std::io::Read + std::io::Seek>(
    comp: &mut cfb::CompoundFile<F>,
) -> anyhow::Result<()> {
    for entry in comp.walk() {
        if entry.is_root() {
            continue;
        }
        let path = entry.path();
        // Depth = number of components minus 1 (root)
        let depth = path.components().count().saturating_sub(1);
        let indent = "  ".repeat(depth);
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());

        if entry.is_storage() {
            println!("{indent}{name}/");
        } else {
            println!("{indent}{name}  ({} bytes)", entry.len());
        }
    }
    Ok(())
}

fn cmd_dump(args: &DumpArgs) -> anyhow::Result<()> {
    let mut comp = open_cfb(&args.file)?;
    let stream_path = Path::new(&args.stream);
    let data = read_stream(&mut comp, stream_path)
        .map_err(|e| anyhow::anyhow!("failed to read stream '{}': {e}", args.stream))?;

    let start = args.offset.unwrap_or(0);
    if start >= data.len() {
        anyhow::bail!(
            "offset {} is beyond stream length {} for '{}'",
            start,
            data.len(),
            args.stream
        );
    }

    if args.blocks {
        // Annotated dump: show block boundaries and decode text blocks
        let blocks = parse_blocks(&data);
        for block in &blocks {
            let block_end = block.header_offset + 4 + block.size as usize;

            // Skip blocks entirely outside the requested window
            if let Some(limit) = args.limit {
                if block.header_offset >= start + limit {
                    break;
                }
            }
            if block_end <= start {
                continue;
            }

            println!(
                "--- block[{}] @ {:#x}: {} ({} bytes) ---",
                block.index,
                block.header_offset,
                block.format_label(),
                block.size
            );

            // Determine the window within this block to dump
            let dump_start = if block.header_offset >= start {
                block.header_offset
            } else {
                start
            };
            let dump_end = match args.limit {
                Some(n) => block_end.min(start + n),
                None => block_end,
            };

            if dump_start < dump_end {
                // Dump header bytes if in range
                let header_end = block.header_offset + 4;
                if dump_start < header_end {
                    print_hex_dump(
                        &data,
                        dump_start,
                        Some(header_end.min(dump_end) - dump_start),
                    );
                }
                // Dump payload bytes
                let payload_start = header_end.max(dump_start);
                if payload_start < dump_end {
                    print_hex_dump(&data, payload_start, Some(dump_end - payload_start));
                }
            }

            // For text blocks, show decoded content
            if !block.is_binary() && !block.payload.is_empty() {
                let decoded = decode_text_block(block.payload);
                println!("    text: {decoded}");
            }
        }
    } else {
        // Plain hex dump
        print_hex_dump(&data, start, args.limit);
    }

    Ok(())
}

fn cmd_blocks(args: &BlocksArgs) -> anyhow::Result<()> {
    let mut comp = open_cfb(&args.file)?;
    let stream_path = Path::new(&args.stream);
    let data = read_stream(&mut comp, stream_path)
        .map_err(|e| anyhow::anyhow!("failed to read stream '{}': {e}", args.stream))?;

    let blocks = parse_blocks(&data);

    if blocks.is_empty() {
        println!("No blocks found in '{}'", args.stream);
        return Ok(());
    }

    match args.block {
        Some(idx) => {
            // Detailed view of a single block
            let block = blocks.get(idx).ok_or_else(|| {
                anyhow::anyhow!("block index {idx} out of range (0..{})", blocks.len())
            })?;

            println!(
                "block[{}] @ offset {:#x}: format={}, flags={:#04x}, payload={} bytes",
                block.index,
                block.header_offset,
                block.format_label(),
                block.flags,
                block.size
            );

            // Full hex dump of payload
            if !block.payload.is_empty() {
                println!();
                print_hex_dump(block.payload, 0, None);
            }

            // Decode text
            if !block.is_binary() && !block.payload.is_empty() {
                let decoded = decode_text_block(block.payload);
                println!("\nDecoded text:");
                println!("{decoded}");
            }
        }
        None => {
            // Summary of all blocks
            println!(
                "{} blocks in '{}' ({} bytes total)",
                blocks.len(),
                args.stream,
                data.len()
            );
            println!();

            for block in &blocks {
                let preview = if block.is_binary() {
                    // Show first few bytes as hex
                    let preview_len = block.payload.len().min(24);
                    let hex: Vec<String> = block.payload[..preview_len]
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect();
                    let mut s = hex.join(" ");
                    if preview_len < block.payload.len() {
                        s.push_str(" ...");
                    }
                    s
                } else {
                    // Show decoded text, truncated
                    let decoded = decode_text_block(block.payload);
                    if decoded.len() > 80 {
                        format!("{}...", &decoded[..80])
                    } else {
                        decoded
                    }
                };

                println!(
                    "  [{:3}] @{:#08x}  {:6}  {:6} bytes  {}",
                    block.index,
                    block.header_offset,
                    block.format_label(),
                    block.size,
                    preview
                );
            }
        }
    }

    Ok(())
}

fn cmd_diff(args: &DiffArgs) -> anyhow::Result<bool> {
    let bytes1 = std::fs::read(&args.file1)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", args.file1.display()))?;
    let bytes2 = std::fs::read(&args.file2)
        .map_err(|e| anyhow::anyhow!("failed to read {}: {e}", args.file2.display()))?;

    if bytes1 == bytes2 {
        println!("Files are identical");
        return Ok(true);
    }

    let first_diff = bytes1
        .iter()
        .zip(bytes2.iter())
        .position(|(a, b)| a != b)
        .unwrap_or_else(|| bytes1.len().min(bytes2.len()));

    println!(
        "Files differ: first difference at byte offset {first_diff:#010x} ({})",
        first_diff
    );
    if bytes1.len() != bytes2.len() {
        println!(
            "  File sizes differ: {} bytes vs {} bytes",
            bytes1.len(),
            bytes2.len()
        );
    }

    let mut comp1 = cfb::CompoundFile::open(std::io::Cursor::new(&bytes1))
        .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", args.file1.display()))?;
    let mut comp2 = cfb::CompoundFile::open(std::io::Cursor::new(&bytes2))
        .map_err(|e| anyhow::anyhow!("failed to open {}: {e}", args.file2.display()))?;

    let entries1 = collect_entries(&mut comp1);
    let entries2 = collect_entries(&mut comp2);

    let all_paths: BTreeSet<String> = entries1.union(&entries2).cloned().collect();

    let mut any_diff = false;

    for path_str in &all_paths {
        // Filter to a single stream if requested
        if let Some(ref filter) = args.stream {
            if path_str != filter {
                continue;
            }
        }

        let in1 = entries1.contains(path_str);
        let in2 = entries2.contains(path_str);

        if !in1 {
            println!("  MISSING in file1: {path_str}");
            any_diff = true;
            continue;
        }
        if !in2 {
            println!("  EXTRA in file1 (missing in file2): {path_str}");
            any_diff = true;
            continue;
        }

        let cfb_path = Path::new(path_str);

        if comp1.is_storage(cfb_path) {
            if args.verbose {
                println!("  storage OK: {path_str}");
            }
            continue;
        }

        let stream1 = read_stream(&mut comp1, cfb_path)
            .map_err(|e| anyhow::anyhow!("failed to read stream {path_str} from file1: {e}"))?;
        let stream2 = read_stream(&mut comp2, cfb_path)
            .map_err(|e| anyhow::anyhow!("failed to read stream {path_str} from file2: {e}"))?;

        if stream1 == stream2 {
            if args.verbose {
                println!("  stream OK: {path_str} ({} bytes)", stream1.len());
            }
            continue;
        }

        any_diff = true;
        let stream_diff_offset = stream1
            .iter()
            .zip(stream2.iter())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| stream1.len().min(stream2.len()));

        println!("  stream DIFFERS: {path_str}");
        if stream1.len() != stream2.len() {
            println!(
                "    length: {} bytes vs {} bytes",
                stream1.len(),
                stream2.len()
            );
        }
        println!(
            "    first diff at stream offset {stream_diff_offset:#010x} ({stream_diff_offset})"
        );
        print_hex_context(&stream1, &stream2, stream_diff_offset);

        if args.blocks {
            diff_blocks(path_str, &stream1, &stream2);
        }
    }

    Ok(!any_diff)
}

fn cmd_diff_semantic(args: &DiffArgs) -> anyhow::Result<bool> {
    let mut options = altium_format::test_utils::CfbSemanticDiffOptions::new();
    if args.case_insensitive_keys {
        options = options.case_insensitive_keys();
    }
    let report = altium_format::test_utils::diff_cfb_files_semantic_with_options(
        &args.file1,
        &args.file2,
        &options,
    )
    .map_err(|e| anyhow::anyhow!("{e}"))?;

    if report.is_identical() {
        println!("No semantic differences.");
        return Ok(true);
    }

    if args.verbose {
        print!("{}", report.render());
    } else {
        print!("{}", report.render_categorized());
    }

    Ok(false)
}

fn print_hex_context(data1: &[u8], data2: &[u8], offset: usize) {
    let window_start = offset.saturating_sub(8);
    let window_end = (offset + 8).min(data1.len().max(data2.len()));

    let slice1 = &data1[window_start..window_end.min(data1.len())];
    let slice2 = &data2[window_start..window_end.min(data2.len())];

    print!("    file1 hex [{window_start:#010x}+]: ");
    for b in slice1 {
        print!("{b:02x} ");
    }
    println!();

    print!("    file2 hex [{window_start:#010x}+]: ");
    for b in slice2 {
        print!("{b:02x} ");
    }
    println!();
}

fn diff_blocks(stream_path: &str, data1: &[u8], data2: &[u8]) {
    let blocks1 = parse_blocks(data1);
    let blocks2 = parse_blocks(data2);

    if blocks1.len() != blocks2.len() {
        println!(
            "    blocks: {} blocks vs {} blocks",
            blocks1.len(),
            blocks2.len()
        );
    }

    let count = blocks1.len().max(blocks2.len());
    for i in 0..count {
        match (blocks1.get(i), blocks2.get(i)) {
            (Some(b1), Some(b2)) => {
                if b1.payload != b2.payload || b1.flags != b2.flags {
                    println!(
                        "    block[{i}] differs in {stream_path}: {}({} bytes) vs {}({} bytes)",
                        b1.format_label(),
                        b1.size,
                        b2.format_label(),
                        b2.size
                    );
                    // Show decoded text for text blocks that differ
                    if !b1.is_binary() {
                        let decoded = decode_text_block(b1.payload);
                        println!("      file1 text: {decoded}");
                    }
                    if !b2.is_binary() {
                        let decoded = decode_text_block(b2.payload);
                        println!("      file2 text: {decoded}");
                    }
                }
            }
            (Some(b1), None) => {
                println!(
                    "    block[{i}] only in file1: {}({} bytes)",
                    b1.format_label(),
                    b1.size
                );
            }
            (None, Some(b2)) => {
                println!(
                    "    block[{i}] only in file2: {}({} bytes)",
                    b2.format_label(),
                    b2.size
                );
            }
            (None, None) => {}
        }
    }
}

fn cmd_cat(args: &CatArgs) -> anyhow::Result<()> {
    let mut comp = open_cfb(&args.file)?;
    let stream_path = Path::new(&args.stream);
    let data = read_stream(&mut comp, stream_path)
        .map_err(|e| anyhow::anyhow!("failed to read stream '{}': {e}", args.stream))?;

    std::io::stdout().write_all(&data)?;
    std::io::stdout().flush()?;

    Ok(())
}
