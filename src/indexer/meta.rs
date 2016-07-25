use std::mem;
use std::slice;

struct IndexMeta {
    dict_size: u64,
    root_ptr: u64,
    term_buffer_size: u64,
    docs_size: u64,
    tfs_size: u64,
    positions_size: u64,
}

impl IndexMeta {
    fn from_bytes(bs: &[u8]) -> &Self {
        assert!(bs.len() >= mem::size_of::<IndexMeta>());
        unsafe { mem::transmute(&bs[0]) }
    }

    fn to_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self as *const _ as *const u8, mem::size_of::<IndexMeta>()) }
    }
}
