pub type TermId = u32;

#[derive(Clone, Debug)]
pub struct Term<'a> {
    pub term: &'a str,
    pub term_id: TermId,
}

