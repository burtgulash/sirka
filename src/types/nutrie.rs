use types::{DocId,TermId};

// TODO packed necessary?
#[repr(C)]
#[derive(Debug)]
pub struct TrieNodeHeader {
    pub postings_ptr: u32, // DOCID
    pub term_ptr: u32,
    pub term_id: u32, // TERMID
    pub num_postings: u32,
    pub num_children: u32,
    pub term_length: u16,
    pub is_word: bool,
}

