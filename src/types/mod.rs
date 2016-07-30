pub use self::meta::IndexMeta;
pub use self::seq::{Sequence,SequenceStorage,SequenceEncoder};
pub use self::nutrie::TrieNodeHeader;
pub use self::postings::{PostingsStore,Postings};

pub mod meta;
pub mod seq;
pub mod nutrie;
pub mod postings;


pub type TermId = u32;
pub type DocId = u64;

#[derive(Clone, Debug)]
pub struct Term {
    pub term: String,
    pub term_id: TermId,
}
