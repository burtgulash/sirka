use types::*;

struct CumSequence<S> {
    seq: S,
    last: Option<DocId>,
}

impl<S: Sequence> CumSequence<S> {
    fn new(mut seq: S) -> Self {
        let last = seq.next();
        CumSequence {
            seq: seq,
            last: last,
        }
    }
}

macro_rules! tryopt {
    ($e:expr) => (match $e {
        Some(value) => value,
        None => return None,
    })
}

impl<S: Sequence> Sequence for CumSequence<S> {
    fn next(&mut self) -> Option<DocId> {
        if let Some(next) = self.seq.next() {
            if let Some(last) = self.last {
                self.last = Some(next);
                return Some(next - last);
            }
        }
        None
    }

    fn current_position(&self) -> Option<usize> {
        Some(tryopt!(self.seq.current_position()) - 1)
    }

    fn remains(&self) -> usize {
        self.seq.remains()
    }

    fn skip_to(&mut self, doc_id: DocId) -> Option<DocId> {
        panic!("Can't use move_to on cumulative sequence");
    }

    fn skip_n(&mut self, n: usize) -> Option<DocId> {
        self.last = self.seq.skip_n(n - 1);
        self.next()
    }

    fn subsequence(&self, start: usize, len: usize) -> Self {
        Self::new(self.seq.subsequence(start, len + 1))
    }
}

#[cfg(test)]
mod tests {
    use super::CumSequence;
    use types::{Sequence,SequenceStorage};

    #[test]
    fn test_cum_sequence() {
        let docs = vec![3,4,7,9,10,14,15,18,25,27,33,50,55,100];
        let subseq_len = 7;
        let mut cumseq1 = CumSequence::new((&docs[..]).to_sequence());
        let mut cumseq = cumseq1.subsequence(3, subseq_len);
        println!("REMAINS: {}", cumseq.remains());
        assert_eq!(subseq_len, cumseq.remains());

        let mut count = 0;
        while let Some(doc) = cumseq.next() {
            count += 1;
            println!("Next doc: {}", doc);
        }
        assert_eq!(subseq_len, count);
    }
}
