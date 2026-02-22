# Altium Designer PCB Pad Binary Format Analysis
## AdvPCB.dll (Delphi x64) - TServerPad Object

### Known Structure (from KiCad parser)

Subrecord 5 - Pad core data (minimum 110 bytes, can be 114, 120, 171, 202+):

```
Offset | Size | Type | Field Name (KiCad)              | Notes
-------|------|------|----------------------------------|---------------------------
0      | 1    | u8   | layer                           | ALTIUM_LAYER enum
1      | 1    | u8   | flags1                          | test_fab_top|tent_bottom|tent_top|is_locked
2      | 1    | u8   | flags2                          | test_fab_bottom
3      | 2    | u16  | net                             | Network index
5      | 2    | u16  | (padding)                       | Skipped
7      | 2    | u16  | component                       | Component index
9      | 4    | u32  | (padding)                       | Skipped
13     | 4    | i32  | position.x                      | Pad X position
17     | 4    | i32  | position.y                      | Pad Y position
21     | 4    | i32  | topsize.x                       | Top layer pad width
25     | 4    | i32  | topsize.y                       | Top layer pad height
29     | 4    | i32  | midsize.x                       | Mid layer pad width
33     | 4    | i32  | midsize.y                       | Mid layer pad height
37     | 4    | i32  | botsize.x                       | Bottom layer pad width
41     | 4    | i32  | botsize.y                       | Bottom layer pad height
45     | 4    | i32  | holesize                        | Hole diameter
49     | 1    | u8   | topshape                        | ALTIUM_PAD_SHAPE (top)
50     | 1    | u8   | midshape                        | ALTIUM_PAD_SHAPE (mid)
51     | 1    | u8   | botshape                        | ALTIUM_PAD_SHAPE (bottom)
52     | 8    | f64  | direction                       | Rotation angle (degrees)
60     | 1    | u8   | plated                          | Plated hole flag (boolean)
```

### **UNKNOWN FIELDS - To Be Reverse Engineered**

#### Priority 1: Single-byte unknowns

```
Offset | Size | Type | Function Address | .NET Property Candidate          | Hypothesis
-------|------|------|------------------|----------------------------------|---------------------------
61     | 1    | u8   | FUN_017c4af0     | HoleType?                       | After plated, before pad_mode
62     | 1    | u8   | (KNOWN)          | pad_mode                        | ALTIUM_PAD_MODE enum
63-85  | 23   | ?    | (MULTIPLE)       | Thermal relief, pad-to-die, etc | Complex region
96     | 1    | u8   | FUN_017caef0     | ???                             | Unknown
97     | 1    | u8   | FUN_017cb5b0     | ???                             | Unknown
98     | 1    | u8   | FUN_017cb710     | ???                             | Unknown
99     | 1    | u8   | FUN_017cb450     | ???                             | Unknown
100    | 1    | u8   | ???              | ???                             | Unknown
101    | 1    | u8   | (KNOWN)          | pastemaskexpansionmode          | ALTIUM_MODE enum
102    | 1    | u8   | FUN_017cb920     | soldermaskexpansionmode?        | Already known in KiCad?
103    | 1    | u8   | FUN_017cb190     | ???                             | Unknown
104    | 1    | u8   | FUN_017cb030     | ???                             | Unknown
105    | 1    | u8   | ???              | ???                             | Unknown
106    | ?    | ?    | ???              | union_index?                    | Unknown
125    | 1    | u8   | FUN_0185e2c0     | ???                             | Unknown
170    | 1    | u8   | FUN_017c8360     | ???                             | Unknown
```

#### Priority 2: 4-byte (i32) unknowns

```
Offset | Size | Type | Function Address | .NET Property Candidate          | Hypothesis
-------|------|------|------------------|----------------------------------|---------------------------
63     | 4    | i32  | FUN_017c5330     | ???                             | Right after pad_mode at 62
78     | 4    | i32  | FUN_017cb240     | Thermal relief?                 | Between thermal and paste/solder
82     | 4    | i32  | FUN_017cb0e0     | ???                             | Unknown
110    | 4    | i32  | FUN_017c3fd0     | ???                             | Between union_index and layer_enum at 114
121    | 4    | i32  | FUN_017cb7c0     | ???                             | Unknown
158    | 4    | i32  | FUN_017c40a0     | ???                             | Unknown
162    | 4    | i32  | FUN_017c8600     | ???                             | Unknown
166    | 4    | i32  | FUN_017c85d0     | ???                             | Unknown
```

#### Priority 3: Hole shape related

```
Offset | Size | Type | Function Address | .NET Property Candidate          | Hypothesis
-------|------|------|------------------|----------------------------------|---------------------------
118    | 1    | u8   | FUN_0185e2b0     | Hole shape related?             | Near hole data
119    | 1    | u8   | FUN_0185e2a0     | Hole shape related?             | Near hole data
```

### .NET Interface IPCB_Pad3 Properties (Reference)

These properties from the .NET interface should map to fields in the binary structure:

- Name ✓ (subrecord 1)
- Plated ✓ (offset 60)
- Rotation ✓ (offset 52-59, direction)
- Layer ✓ (offset 0)
- Mode ✓ (offset 62, pad_mode)
- HoleSize ✓ (offset 45-48)
- **HoleType** ❌ (UNKNOWN - possibly offset 61?)
- **HoleRotation** ❌ (UNKNOWN - in extended data >= 114 bytes)
- **HoleWidth** ❌ (UNKNOWN - for slotted holes)
- **PinPackageLength** ❌ (UNKNOWN)
- **JumperID** ❌ (UNKNOWN)
- **DaisyChainStyle** ❌ (UNKNOWN)
- **XPadOffsetAllLayers** ❌ (UNKNOWN)
- **YPadOffsetAllLayers** ❌ (UNKNOWN)
- IsTopPasteEnabled ✓ (in flags or mode)
- IsBottomPasteEnabled ✓ (in flags or mode)
- IsTentingTop ✓ (offset 1, flags1 & 0x20)
- IsTentingBottom ✓ (offset 1, flags1 & 0x40)
- IsTestPoint_Top ✓ (offset 1, flags1 & 0x80)
- IsTestPoint_Bottom ✓ (offset 2, flags2 & 0x01)
- IsAssyTestPoint_Top ❌ (UNKNOWN)
- IsAssyTestPoint_Bottom ❌ (UNKNOWN)
- **SolderMaskExpansionFromHoleEdge** ❌ (UNKNOWN)
- **PropagationDelay** ❌ (UNKNOWN - in extended data >= 202 bytes)

### Extended Subrecord 5 Sizes

- **110 bytes**: Basic pad data
- **114 bytes**: Adds `holerotation` (f64) at offset 106
- **120 bytes**: Adds `tolayer` (u8) and `fromlayer` (u8) at offsets 114, 117
- **171 bytes**: Unknown additional fields
- **202+ bytes**: Adds `pad_to_die_length` and `pad_to_die_delay`

### Key Functions to Decompile

#### Main I/O Functions:
- **FUN_0184ad40**: Pad binary reader (reads all fields in order)
- **FUN_01858be0**: Pad binary writer (writes all fields in order)

#### Priority 1 Getter/Setters:
1. FUN_017c4af0 (offset 61, u8)
2. FUN_017caef0 (offset 96, u8)
3. FUN_017cb5b0 (offset 97, u8)
4. FUN_017cb710 (offset 98, u8)
5. FUN_017cb450 (offset 99, u8)
6. FUN_017cb920 (offset 102, u8)
7. FUN_017cb190 (offset 103, u8)
8. FUN_017cb030 (offset 104, u8)
9. FUN_0185e2c0 (offset 125, u8)
10. FUN_017c8360 (offset 170, u8)

#### Priority 2 Getter/Setters:
11. FUN_017c5330 (offset 63, i32)
12. FUN_017cb240 (offset 78, i32)
13. FUN_017cb0e0 (offset 82, i32)
14. FUN_017c3fd0 (offset 110, i32)
15. FUN_017cb7c0 (offset 121, i32)
16. FUN_017c40a0 (offset 158, i32)
17. FUN_017c8600 (offset 162, i32)
18. FUN_017c85d0 (offset 166, i32)

#### Priority 3 Getter/Setters:
19. FUN_0185e2b0 (offset 118, u8)
20. FUN_0185e2a0 (offset 119, u8)

### Next Steps

1. Use Ghidra to decompile each function
2. Identify if it's a getter (reads from object+offset) or setter (writes to object+offset)
3. Look for string references, cross-references, and context
4. Match against .NET interface property names
5. Document the Delphi property name and purpose
