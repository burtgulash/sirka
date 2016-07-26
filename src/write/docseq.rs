use write::*;
use types::*;

pub trait Sequence<'a> {
    type Slider: SequenceSlider + 'a;
    fn slider(&self) -> Self::Slider;
}

pub trait SequenceSlider: Clone {
    fn next(&mut self) -> Option<DocId>;
    fn skip_to(&mut self, doc_id: DocId) -> Option<DocId>;
    fn skip_n(&mut self, n: usize) -> Option<DocId>;
}

impl<'a> Sequence<'a> for &'a [DocId] {
    type Slider = SliceSequenceSlider<'a>;
    fn slider(&self) -> Self::Slider {
        SliceSequenceSlider::new(*self)
    }
}

#[derive(Clone)]
pub struct SliceSequenceSlider<'a> {
    seq: &'a [DocId],
    position: usize,
}

impl<'a> SliceSequenceSlider<'a> {
    fn new(seq: &'a [DocId]) -> Self {
        SliceSequenceSlider {
            seq: seq,
            position: 0,
        }
    }

    fn return_at_current(&self) -> Option<DocId> {
        if self.position < self.seq.len() {
            Some(self.seq[self.position - 1])
        } else {
            None
        }
    }
}

impl<'a> SequenceSlider for SliceSequenceSlider<'a> {
    fn next(&mut self) -> Option<DocId> {
        self.skip_n(1)
    }

    fn skip_to(&mut self, doc_id: DocId) -> Option<DocId> {
        while self.position < self.seq.len()
           && self.seq[self.position] < doc_id
        { self.position += 1; }
        self.return_at_current()
    }

    fn skip_n(&mut self, n: usize) -> Option<DocId> {
        self.position += n;
        self.return_at_current()
    }
}
