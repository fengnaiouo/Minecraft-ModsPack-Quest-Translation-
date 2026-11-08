//! QTrans core: Minecraft modpack quest text extraction, translation and repacking.
//!
//! Supported inputs:
//! - CurseForge / Modrinth style modpack zips (`overrides/...`)
//! - already extracted pack folders
//!
//! Supported text sources:
//! - FTB Quests `config/ftbquests/quests/**/*.snbt` (embedded strings)
//! - KubeJS resource lang files `kubejs/assets/*/lang/en_us.json`
//! - player-facing strings in KubeJS scripts

pub mod apply;
pub mod ids;
pub mod javascript;
pub mod lang;
pub mod model;
pub mod package;
pub mod project;
pub mod scan;
pub mod snbt;

pub use model::{Manifest, ProjectConfig, Translation, Unit, UnitKind};
pub use project::Project;

pub const DEFAULT_TARGET: &str = "zh_tw";
