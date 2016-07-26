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
