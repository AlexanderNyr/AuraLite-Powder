//! Elements crate - Defines Element trait, registry, nuclear mechanics

pub mod element_trait;
pub mod nuclear;
pub mod reaction_table;
pub mod registry;

pub use element_trait::{
    Element, ElementDef, ReactionCtx, ReactionEvent, RgbaColor, UpdateCtx, UpdateResult,
};
pub use nuclear::{criticality, decay, fission, fusion};
pub use reaction_table::{ReactionOutcome, ReactionPair, ReactionTable};
pub use registry::{
    all_definitions, color_for_id, definition_ref, density_for_id, get_definition, name_for_id,
};
