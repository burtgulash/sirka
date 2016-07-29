pub use self::meta::IndexMeta;
pub use self::docseq::{Sequence};
pub use self::nutrie::TrieNodeHeader;

pub mod meta;
pub mod docseq;
pub mod nutrie;


pub type TermId = u32;
pub type DocId = u64;

#[derive(Clone, Debug)]
pub struct Term {
    pub term: String,
    pub term_id: TermId,
}
