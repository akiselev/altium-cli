#!/usr/bin/env bash
set -euo pipefail

CLI="/home/kiselev/git/altium-cli-simplified/target/release/altium-cli"
SCHLIB_DIR="/home/kiselev/git/altium-cli-simplified/data/schlib"
SAVEAS_DIR="/home/kiselev/git/altium-cli-simplified/data/schlib-saveas"
REPORT_DIR="/home/kiselev/git/altium-cli-simplified/docs/schlib-diff"

mkdir -p "$SAVEAS_DIR" "$REPORT_DIR"

FILES=(
  "NEUACTION.SchLib"
  "Parts_Library.SchLib"
  "RAM.SchLib"
  "Resistors_Caps.SchLib"
  "ryankurte-ARJM11.SchLib"
  "ryankurte-ATSAMD21G.SchLib"
  "ryankurte-EFM32GG12B8xx.SchLib"
  "ryankurte-electronpowered.SchLib"
  "S32K.SchLib"
  "Sika_revb.SchLib"
  "SIM808.SchLib"
  "SMotlaq-Schem_lib.SchLib"
  "Standard.SchLib"
  "STM32.SchLib"
  "Switches.SchLib"
  "Synthiam.SchLib"
  "TinyShuffle.SchLib"
  "Transistors.SchLib"
  "vpodlesnyi-Amplifier.SchLib"
  "vpodlesnyi-Connector.SchLib"
  "vpodlesnyi-Driver.SchLib"
  "vpodlesnyi-GOSTAmplifier.SchLib"
  "vpodlesnyi-GOSTDiode.SchLib"
  "vpodlesnyi-GOSTInductor.SchLib"
)

for filename in "${FILES[@]}"; do
  echo "========================================="
  echo "Processing: $filename"
  echo "========================================="

  original="$SCHLIB_DIR/$filename"
  saved="$SAVEAS_DIR/$filename"
  report="$REPORT_DIR/$filename.md"

  # Step 1: Get version
  echo "  Getting version..."
  version_output=$("$CLI" get version "$original" 2>&1) || version_output="ERROR: $version_output"
  echo "  Version: $version_output"

  # Step 2: Save-as
  echo "  Running save-as..."
  saveas_output=$("$CLI" save-as "$original" "$saved" 2>&1) && saveas_success=true || saveas_success=false
  if $saveas_success; then
    saveas_result="Success"
    echo "  Save-as: Success"
  else
    saveas_result="Error: $saveas_output"
    echo "  Save-as: FAILED"
  fi

  # Step 3: CFB diff (only if save-as succeeded)
  if $saveas_success; then
    echo "  Running cfb diff..."
    diff_output=$("$CLI" cfb diff "$original" "$saved" --blocks -v 2>&1) || diff_output="DIFF ERROR: $diff_output"
    echo "  Diff: done"
  else
    diff_output="N/A (save-as failed)"
  fi

  # Step 4: Write report
  cat > "$report" <<REPORT_EOF
# $filename

## Version
Original: $version_output

## Save-As Result
$saveas_result

## CFB Diff
\`\`\`
$diff_output
\`\`\`
REPORT_EOF

  echo "  Report written to: $report"
  echo ""
done

echo "All files processed."
