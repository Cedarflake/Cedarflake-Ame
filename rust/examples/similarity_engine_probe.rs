#[path = "similarity_engine_probe/mod.rs"]
mod similarity_engine_probe;

fn main() {
    similarity_engine_probe::quality::compare_hashes();
    similarity_engine_probe::index::compare_indexes();
}
