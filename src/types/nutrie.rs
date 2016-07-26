use types::{DocId,TermId};

// TODO packed necessary?
#[repr(C)]
#[derive(Debug)]
pub struct TrieNodeHeader {
    pub postings_ptr: DocId, // DOCID
    pub term_ptr: u32,
    pub term_id: u32, // TERMID
    pub num_children: TermId,
    pub term_length: u8,
    pub is_word: bool,
}

