//! PCB constants from Altium SDK.

/// Rule kind string IDs — `cRuleIdStrings` from SDK.
///
/// Index matches `TRuleKind` discriminant. Used in `Rules6` parametric `RULEKIND=` field.
pub const RULE_ID_STRINGS: &[&str; 52] = &[
    "Clearance",            // 0
    "ParallelSegment",      // 1
    "Width",                // 2
    "Length",                // 3
    "MatchedLengths",       // 4
    "StubLength",           // 5
    "PlaneConnect",         // 6
    "RoutingTopology",      // 7
    "RoutingPriority",      // 8
    "RoutingLayers",        // 9
    "RoutingCorners",       // 10
    "RoutingVias",          // 11
    "PlaneClearance",       // 12
    "SolderMaskExpansion",  // 13
    "PasteMaskExpansion",   // 14
    "ShortCircuit",         // 15
    "UnRoutedNet",          // 16
    "ViasUnderSMD",         // 17
    "MaximumViaCount",      // 18
    "MinimumAnnularRing",   // 19
    "PolygonConnect",       // 20
    "AcuteAngle",           // 21
    "RoomDefinition",       // 22
    "SMDToCorner",          // 23
    "ComponentClearance",   // 24
    "ComponentOrientations", // 25
    "PermittedLayers",      // 26
    "NetsToIgnore",         // 27
    "SignalStimulus",       // 28
    "OvershootFalling",     // 29
    "OvershootRising",      // 30
    "UndershootFalling",    // 31
    "UndershootRising",     // 32
    "MaxMinImpedance",      // 33
    "SignalTopValue",       // 34
    "SignalBaseValue",      // 35
    "FlightTimeRising",     // 36
    "FlightTimeFalling",    // 37
    "LayerStack",           // 38
    "SlopeRising",          // 39
    "SlopeFalling",         // 40
    "SupplyNets",           // 41
    "HoleSize",             // 42
    "Testpoint",            // 43
    "TestPointUsage",       // 44
    "UnConnectedPin",       // 45
    "SMDToPlane",           // 46
    "SMDNeckDown",          // 47
    "LayerPairs",           // 48
    "FanoutControl",        // 49
    "Height",               // 50
    "DiffPairsRouting",     // 51
];

/// Objects that use WideString encoding.
pub const WIDE_STRING_OBJECTS: &[&str] = &["Text", "Dimension", "Coordinate", "Component"];

/// No-reference sentinel value for u16 index fields.
pub const NO_REF: u16 = 0xFFFF;
