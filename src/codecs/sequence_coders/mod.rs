pub use self::delta::DeltaEncoder;
pub use self::cum::CumEncoder;
pub use self::ascending::{AscendingEncoder,AscendingDecoder};
pub use self::merge::MergeEncoder;

pub type DeltaDecoder<S> = CumEncoder<S>;
pub type CumDecoder<S> = DeltaEncoder<S>;

pub mod delta;
pub mod cum;
pub mod ascending;
pub mod merge;
