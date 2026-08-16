from pathlib import Path

path = Path("crates/altium-cli/src/sync_v2.rs")
text = path.read_text()
block = '''struct MaterializedDocument {
    dump: String,
    bytes: Vec<u8>,
}

'''
while block + block in text:
    text = text.replace(block + block, block)
path.write_text(text)
