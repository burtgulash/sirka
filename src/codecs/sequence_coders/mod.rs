use self::delta::DeltaEncoder;
use self::cum::CumEncoder;
use self::ascending::{AscendingEncoder,AscendingDecoder};
use self::merge::MergeEncoder;

pub type DeltaDecoder<S> = CumEncoder<S>;
pub type CumDecoder<S> = DeltaEncoder<S>;

pub mod delta;
pub mod cum;
pub mod ascending;
pub mod merge;
