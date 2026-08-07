mod cli;
mod lsp;
mod support;
mod java_constants {
    include!(concat!(env!("OUT_DIR"), "/java_constants.rs"));
}
mod java_queries {
    include!(concat!(env!("OUT_DIR"), "/java_queries.rs"));
}

pub use cli::main;
