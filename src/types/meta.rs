use std::mem;
use std::slice;

#[derive(Debug)]
pub struct IndexMeta {
    pub dict_size: u64,
    pub root_ptr: u64,
    pub term_buffer_size: u64,
    pub docs_size: u64,
    pub tfs_size: u64,
    pub positions_size: u64,
}

impl IndexMeta {
    pub fn from_bytes(bs: &[u8]) -> &Self {
        assert!(bs.len() >= mem::size_of::<IndexMeta>());
        unsafe { mem::transmute(&bs[0]) }
    }

    pub fn to_bytes(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self as *const _ as *const u8, mem::size_of::<IndexMeta>()) }
    }
}
