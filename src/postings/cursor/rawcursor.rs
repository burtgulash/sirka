use types::*;
use postings::{Postings,PostingsCursor,Sequence};

pub struct RawCursor<DS: Sequence, TS: Sequence, PS: Sequence> {
    postings: Postings<DS, TS, PS>,
    ahead: usize,
    term_id: TermId,
    advanced: bool,
}

impl<DS: Sequence, TS: Sequence, PS: Sequence> RawCursor<DS, TS, PS> {
    pub fn new(mut postings: Postings<DS, TS, PS>, term_id: TermId) -> Self {
        // prime tfs, because it has one more element at the end
        postings.tfs.next();
        RawCursor {
            postings: postings,
            term_id: term_id,
            ahead: 0,
            advanced: false,
        }
    }
}

impl<DS: Sequence, TS: Sequence, PS: Sequence> PostingsCursor for RawCursor<DS, TS, PS> {
    type DS = DS;
    type TS = TS;
    type PS = PS;

    fn remains(&self) -> usize {
        self.postings.docs.remains()
    }

    fn term_id(&self) -> TermId {
        self.term_id
    }

    fn current(&self) -> Option<DocId> {
        Some(self.postings.docs.current())
    }

    fn advance(&mut self) -> Option<DocId> {
        self.advanced = true;
        self.ahead += 1;
        self.postings.docs.next() 
    }

    fn advance_to(&mut self, align_to: DocId) -> Option<DocId> {
        let (skipped, x) = self.postings.docs.skip_to(align_to);
        self.ahead += skipped;
        x
    }

    fn catch_up(&mut self, positions_dst: &mut Vec<DocId>) -> DocId {
        assert!(self.advanced);
        self.advanced = false;

        //println!("DOCPTR: {}, TFPTR: {}", self.ptr.docs, self.ptr.tfs);
        // Align tfs to docs
        let start_tf = self.postings.tfs.skip_n(self.ahead - 1).unwrap();
        let next_tf = self.postings.tfs.next().unwrap();
        self.ahead = 0;

        let tf = next_tf - start_tf;
        let mut positions = self.postings.positions.subsequence(start_tf as usize, tf as usize);
        while let Some(position) = positions.next() {
            positions_dst.push(position);
        }

        tf
    }
}



#[cfg(test)]
mod tests {
    use types::*;
    use super::*;
    use postings::{Postings,Sequence,SequenceStorage};

    #[test]
    fn test_cursor() {
        let ps = Postings {
            docs: vec![3,5],
            tfs:  vec![0, 3, 8],
            positions: vec![3,3,3,5,5,5,5,5],
        };
        let seqs = Postings {
            docs: (&ps.docs).to_sequence(),
            tfs: (&ps.tfs).to_sequence(),
            positions: (&ps.positions).to_sequence(),
        };
        let mut cur = RawCursor::new(seqs);
        while let Some(doc_id) = cur.advance() {
            println!("AHEAD: {}", cur.ahead);
            let mut positions = Vec::new();
            let tf = cur.catch_up(&mut positions);
            println!("DOC: {}, TF: {}, POSITIONS: {:?}", doc_id, tf, positions);
        }
        println!("---");
    }

    #[test]
    fn test_cursor2() {
        let ps = Postings {
            docs: vec![1],
            tfs:  vec![0, 1],
            positions: vec![1337],
        };
        let seqs = Postings {
            docs: (&ps.docs).to_sequence(),
            tfs: (&ps.tfs).to_sequence(),
            positions: (&ps.positions).to_sequence(),
        };
        let mut cur = RawCursor::new(seqs);
        while let Some(doc_id) = cur.advance() {
            let mut positions = Vec::new();
            let tf = cur.catch_up(&mut positions);
            println!("DOC: {}, TF: {}, POSITIONS: {:?}", doc_id, tf, positions);
        }
        println!("---");
    }
}
