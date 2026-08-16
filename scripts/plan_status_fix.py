from pathlib import Path

path = Path("crates/altium-sync/src/plan.rs")
text = path.read_text()
old = '''    pub fn has_changes(&self) -> bool {
        !matches!(self.patch, ArtifactPatch::None)
    }
'''
new = '''    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty() || !matches!(self.patch, ArtifactPatch::None)
    }
'''
if old in text:
    text = text.replace(old, new, 1)
elif new not in text:
    raise SystemExit("PlanBundle::has_changes implementation not found")
path.write_text(text)
