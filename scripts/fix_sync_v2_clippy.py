from pathlib import Path

path = Path("crates/altium-sync/src/planner.rs")
text = path.read_text()
old = '''    addresses
        .into_iter()
        .filter_map(|address| {
            let before = current.get(&address).copied();
            let after = desired.get(&address).copied();
            if before.map(|resource| &resource.fingerprint)
                == after.map(|resource| &resource.fingerprint)
            {
                return None;
            }
            let disposition = match direction {
                PlanDirection::Compile => ChangeDisposition::SourceOnly,
                PlanDirection::Dump => ChangeDisposition::DocumentOnly,
            };
            let exemplar = after.or(before)?;
            Some(SemanticChange {
                binding: BindingId::mint(),
                resource_kind: exemplar.kind.clone(),
                resource_key: exemplar.key.clone(),
                disposition,
                change_kind: change_kind(before, after),
                source_before: None,
                source_after: None,
                document_before: before.map(|resource| resource.fingerprint.clone()),
                document_after: after.map(|resource| resource.fingerprint.clone()),
                reason: Some("initial adoption; no synchronization baseline exists".to_string()),
            })
        })
        .collect()
'''
new = '''    let mut changes = Vec::new();
    for address in addresses {
        let before = current.get(&address).copied();
        let after = desired.get(&address).copied();
        if before.map(|resource| &resource.fingerprint)
            == after.map(|resource| &resource.fingerprint)
        {
            continue;
        }
        let disposition = match direction {
            PlanDirection::Compile => ChangeDisposition::SourceOnly,
            PlanDirection::Dump => ChangeDisposition::DocumentOnly,
        };
        let exemplar = after
            .or(before)
            .expect("address originated from the union of current and desired snapshots");
        changes.push(SemanticChange {
            binding: BindingId::mint(),
            resource_kind: exemplar.kind.clone(),
            resource_key: exemplar.key.clone(),
            disposition,
            change_kind: change_kind(before, after),
            source_before: None,
            source_after: None,
            document_before: before.map(|resource| resource.fingerprint.clone()),
            document_after: after.map(|resource| resource.fingerprint.clone()),
            reason: Some("initial adoption; no synchronization baseline exists".to_string()),
        });
    }
    changes
'''
if old in text:
    path.write_text(text.replace(old, new, 1))
elif new not in text:
    raise SystemExit("bootstrap_changes block not found")
