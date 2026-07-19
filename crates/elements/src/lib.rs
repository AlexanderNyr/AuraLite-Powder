//! Elements crate - Defines Element trait, registry, nuclear mechanics

pub mod element_trait;
pub mod registry;
pub mod reaction_table;
pub mod nuclear;

pub use element_trait::{Element, UpdateCtx, ReactionCtx, ReactionEvent, UpdateResult, ElementDef, RgbaColor};
pub use registry::{get_definition, all_definitions, color_for_id, name_for_id, density_for_id};
pub use reaction_table::{ReactionTable, ReactionOutcome, ReactionPair};
pub use nuclear::{fission, fusion, decay, criticality};
