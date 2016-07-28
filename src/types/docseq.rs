use std::io;
use types::*;

pub trait Sequence {
    type Slider: SequenceSlider;
    fn slider(&self, start: usize, len: usize) -> Self::Slider;
}

pub trait SequenceSlider: Clone {
    fn next(&mut self) -> Option<DocId>;
    fn skip_to(&mut self, doc_id: DocId) -> Option<DocId>;
    fn skip_n(&mut self, n: usize) -> Option<DocId>;
    fn index(&self) -> usize;
}

pub trait SequenceWriter {
    fn write_doc(&mut self, doc_id: DocId) -> io::Result<()>;
    fn finish_docs(&mut self) -> io::Result<()>;
    //fn write_index(&mut self) -> io::Result<()>;
}
