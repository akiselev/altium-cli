# Low-Level API Implementation Notes

## Milestone 1: Error Types Expansion

- No existing code referenced `InvalidParamValue(String)` or `BinaryParsingError(String)` outside the enum definition, so the signature changes were non-breaking in practice.
- Error variant display strings match the design doc exactly (e.g., `"Unknown binary code in schematic block: 0x{0:02X}"` with hex formatting).
- `UnknownRecordType(i32)` kept unchanged since it was already in the correct form.
- All variants use `thiserror` `#[error(...)]` for `Display` impl — no manual `Display` implementations needed.
