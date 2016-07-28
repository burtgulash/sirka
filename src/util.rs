use std::{slice,mem};

pub fn common_prefix_len(a: &str, b: &str) -> usize {
    a.chars().zip(b.chars())
        .take_while(|&(ac, bc)| { ac == bc })
        .fold(0, |acc, (x, _)| acc + x.len_utf8())
}

pub fn first_letter(s: &str) -> u32 {
    s.chars().take(1).next().unwrap() as u32
}

pub fn typed_to_bytes<T>(slice: &[T]) -> &[u8] {
    unsafe {
        slice::from_raw_parts(slice.as_ptr() as *const u8, slice.len() * mem::size_of::<T>())
    }
}

pub fn bytes_to_typed<T>(buf: &[u8]) -> &[T] {
    unsafe {
        slice::from_raw_parts(buf.as_ptr() as *const T, buf.len() / mem::size_of::<T>())
    }
}

pub fn align_to(n: usize, alignment: usize) -> usize {
    (alignment - 1) - (n + alignment - 1) % alignment
}

pub fn is_sorted_ascending<T: PartialOrd>(seq: &[T]) -> bool {
    if seq.len() < 1 {
        return true;
    }
    let mut previous = &seq[0];
    for item in seq {
        if previous > item {
            return false;
        }
        previous = item;
    }
    return true;
}
