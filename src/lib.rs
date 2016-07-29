pub mod write;
pub mod read;
pub mod types;
pub mod util;

pub use read::{StaticTrie,SliceSequence};
pub use write::{TermBuf,create_trie};
pub use types::*;
pub use util::bytes_to_typed;
