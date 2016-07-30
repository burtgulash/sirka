pub type TermId = u32;
pub type DocId = u64;

#[derive(Clone, Debug)]
pub struct Term {
    pub term: String,
    pub term_id: TermId,
}
