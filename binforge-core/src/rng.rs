//! M1/AR9: every puzzle's stream is derived from (batch seed, index,
//! attempt), never from shared mutable state, so a puzzle is identical
//! whether it was generated sequentially or on any core, and a single
//! failing puzzle can be regenerated on its own (L3).
