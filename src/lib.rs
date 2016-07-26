pub mod write;
pub mod read;
pub mod types;
mod util;

pub use read::StaticTrie;
pub use write::{TermBuf,create_trie};
pub use types::*;
