mod names;
pub(crate) mod references;
pub(crate) mod sources;

pub use references::collect_dependencies;
pub use sources::load_modules;
