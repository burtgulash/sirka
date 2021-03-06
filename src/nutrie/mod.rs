pub mod read;
pub mod write;

pub use self::write::*;
pub use self::read::*;
use types::{DocId,TermId};

// TODO packed necessary?
#[repr(C)]
#[derive(Debug)]
pub struct TrieNodeHeader {
    pub num_postings: u64,
    pub postings_ptr: DocId,
    pub term_ptr: u32,
    pub term_id: TermId, // TERMID
    pub num_children: u32,
    pub term_length: u16,
}

