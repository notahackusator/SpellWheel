use std::io::Cursor;
use fstools_formats::bnd4::BND4;
use fstools_formats::tpf::TPF;

pub struct ReadSuccess {
    pub bnd: BND4,
    pub tpf: TPF,
    pub tpf_cursor: Cursor<Vec<u8>>,
}

impl ReadSuccess {
    pub fn new(bnd: BND4, tpf: TPF, tpf_cursor: Cursor<Vec<u8>>) -> Self {
        Self {
            bnd,
            tpf,
            tpf_cursor,
        }
    }
}