// Locking, visibility, and read-only state parameters.
//
// These constants control whether objects can be moved, selected, edited,
// or are visible in the schematic editor.

/// Object is locked (cannot be moved/deleted).
///
/// **Wire type:** bool
/// **Used by:** Junction (RECORD=29)
pub const LOCKED: &str = "Locked";

/// Object is graphically locked (cannot be moved).
///
/// **Wire type:** bool
/// **Used by:** all GraphicalObjects
///
/// **Gotcha:** exported but **always reset to false on import** (deprecated).
pub const GRAPHICALLY_LOCKED: &str = "GraphicallyLocked";

/// Designator text is locked (cannot be changed).
///
/// **Wire type:** bool
/// **Used by:** Component (RECORD=1), harness objects
pub const DESIGNATOR_LOCKED: &str = "DesignatorLocked";

/// Part ID is locked.
///
/// **Wire type:** bool
/// **Used by:** Component (RECORD=1)
///
/// Defaults to `DesignatorLocked` value if absent.
pub const PART_ID_LOCKED: &str = "PartIDLocked";

/// Object is hidden (not rendered).
///
/// **Wire type:** bool
/// **Used by:** SheetFileName (RECORD=33), SheetName (RECORD=32),
/// Parameter (RECORD=41)
///
/// For Pin objects, use bit 2 of `PinConglomerate` instead.
pub const IS_HIDDEN: &str = "IsHidden";

/// Object is not accessible/selectable.
///
/// **Wire type:** bool
/// **Used by:** all DataObjects
///
/// **Gotcha:** stored **inverted** -- `true` in file means NOT accessible.
/// Note the intentional typo in the key name (single 's' in "Accesible").
pub const IS_NOT_ACCESSIBLE: &str = "IsNotAccesible";

/// Read-only state for parameters (TParameter_ReadOnlyState enum).
///
/// **Wire type:** u8
/// **Used by:** Parameter (RECORD=41)
///
/// Values: 0=None (fully editable), 1=Name, 2=Value, 3=NameAndValue.
pub const READ_ONLY_STATE: &str = "ReadOnlyState";

/// Object selection state.
///
/// **Wire type:** bool
/// **Used by:** various objects
pub const SELECTION: &str = "Selection";

/// 8-bit bitmask for selection memory group membership.
///
/// **Wire type:** u8
/// **Used by:** all GraphicalObjects
pub const SELECTION_MEMORY: &str = "SelectionMemory";

/// Skip this object during load.
///
/// **Wire type:** bool
/// **Used by:** all DataObjects
///
/// Only written when true.
pub const IGNORE_ON_LOAD: &str = "IgnoreOnLoad";

/// Disable auto-positioning.
///
/// **Wire type:** bool
/// **Used by:** Parameter (RECORD=41), Designator (RECORD=34)
///
/// **Gotcha:** stored inverted from "AutoPosition".
pub const NOT_AUTO_POSITION: &str = "NotAutoPosition";

/// Override the not-auto-position flag.
///
/// **Wire type:** bool
/// **Used by:** Parameter (RECORD=41), Designator (RECORD=34)
pub const OVERRIDE_NOT_AUTO_POSITION: &str = "OverrideNotAutoPosition";

/// Object is active.
///
/// **Wire type:** bool
/// **Used by:** Implementation (RECORD=45)
pub const IS_ACTIVE: &str = "IsActive";

/// Implementation is current (selected).
///
/// **Wire type:** bool
/// **Used by:** Implementation (RECORD=45)
pub const IS_CURRENT: &str = "IsCurrent";
