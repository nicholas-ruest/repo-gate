//! Durable persistence: SQLite pool, migrations, and sqlx-backed stores.

pub mod cache;
pub mod checkpoint_store;
pub mod job_store;
pub mod module_store;
pub mod pool;

pub use cache::AnalysisCacheStore;
pub use checkpoint_store::SqlxCheckpointStore;
pub use job_store::SqlxAssessmentJobStore;
pub use module_store::SqlxModuleAssessmentStore;
pub use pool::create_pool;
