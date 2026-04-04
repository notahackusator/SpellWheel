use std::ptr::NonNull;
use pmod::fmg::MsgRepository;

#[derive(Clone, Debug, PartialEq)]
pub struct Spell {
    index: i32,
    id: u32,
    name: String,
}

impl Spell {
    pub fn try_new(index: i32, id: u32) -> Option<Self> {
        Self::get_name(id).map(|name| Self {
            index,
            id,
            name
        })
    }
    
    pub fn index(&self) -> i32 {
        self.index
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn get_name(spell_id: u32) -> Option<String> {
        const BASE_GAME_SPELL_NAME: u32 = 10;
        const DLC_SPELL_NAME: u32 = 319;

        unsafe {
            read_utf16_string(MsgRepository::get_msg(
                0, BASE_GAME_SPELL_NAME, spell_id
            )).or(read_utf16_string(MsgRepository::get_msg(
                0, DLC_SPELL_NAME, spell_id
            )))
        }
    }
}

unsafe fn read_utf16_string(ptr: Option<NonNull<u16>>) -> Option<String> {
    ptr.map(|ptr| {
        let mut len = 0;
        let mut p = ptr.as_ptr();

        while *p != 0 {
            len += 1;
            p = p.add(1);
        }

        let slice = std::slice::from_raw_parts(ptr.as_ptr(), len);

        String::from_utf16_lossy(slice)
    })
}