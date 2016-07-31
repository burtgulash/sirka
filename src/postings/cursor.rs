use types::*;
use postings::{Postings,Sequence};

pub trait PostingsCursor<DS, TS, PS> {
    fn advance(&mut self) -> Option<DocId>;
    fn advance_to(&mut self, doc_id: DocId) -> Option<DocId>;
    fn current_positions(&mut self) -> PS;
    fn current(&self) -> DocId;
    fn remains(&self) -> usize;
}

impl<DS: Sequence, TS: Sequence, PS: Sequence> PostingsCursor<DS, TS, PS> for SimpleCursor<DS, TS, PS> {
    fn remains(&self) -> usize {
        self.postings.docs.remains()
    }

    fn current(&self) -> DocId {
        self.current
    }

    fn advance(&mut self) -> Option<DocId> {
        if let Some(next_doc_id) = self.postings.docs.next() {
            self.ptr.docs += 1;
            self.current = next_doc_id;

            assert_eq!(self.postings.docs.next_position() - 1, self.ptr.docs);
            Some(next_doc_id)
        } else {
            None
        }
    }

    fn advance_to(&mut self, align_to: DocId) -> Option<DocId> {
        if self.current < align_to {
            if let (Some(doc_id), n_skipped) = self.postings.docs.skip_to(align_to) {
                self.ptr.docs += n_skipped;
                self.current = doc_id;
            } else {
                return None;
            }
        }

        assert!(self.current >= align_to);
        Some(self.current)
    }

    fn current_positions(&mut self) -> PS {
        // Align tfs to docs
        let tf = self.postings.tfs.skip_n(self.ptr.docs - self.ptr.tfs).unwrap();
        self.ptr.tfs = self.ptr.docs;

        // '-1' because tfs sequence has one more element from the sequence
        // of next term in sequence
        assert_eq!(self.postings.tfs.remains() - 1, self.postings.docs.remains());

        // Tfs must have one more element than docs at the end. So that you can take difference
        // between 'next' and 'previous' tfs
        let next_tf = self.postings.tfs.next().unwrap();
        self.ptr.tfs += 1;

        // TODO current_tf is currently unused
        self.current_tf = next_tf;

        // TODO assign new subsequence to self.postings.positions to avoid skipping over the same
        // elements in next round
        let positions = self.postings.positions.subsequence(tf as usize, (next_tf - tf) as usize);

        positions
    }
}

pub struct SimpleCursor<DS: Sequence, TS: Sequence, PS: Sequence> {
    postings: Postings<DS, TS, PS>,
    ptr: Postings<usize, usize, usize>,
    current: DocId,
    current_tf: DocId,

    i: usize,
    term_id: TermId,
}

impl<DS: Sequence, TS: Sequence, PS: Sequence> SimpleCursor<DS, TS, PS> {
    pub fn new(mut postings: Postings<DS, TS, PS>, doc_ptr: usize, index: usize, term_id: TermId) -> Self {
        let first_doc = postings.docs.next().unwrap();
        let first_tf = postings.tfs.next().unwrap();

        SimpleCursor {
            postings: postings,
            ptr: Postings {
                docs: doc_ptr + 1,
                tfs: doc_ptr + 1,
                positions: 0,
            },
            current: first_doc,
            current_tf: first_tf,
            i: index,
            term_id: term_id,
        }
    }
}
