pub use meta::*;
pub use docseq::*;
pub mod meta;
pub mod docseq;


pub type TermId = u32;
pub type DocId = u32;

#[derive(Clone, Debug)]
pub struct Term {
    pub term: String,
    pub term_id: TermId,
}
