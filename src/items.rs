use std::ptr::NonNull;
use eldenring::cs::{EquipParamGoods, SoloParamRepository};
use pmod::fmg::MsgRepository;

#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    index: i32,
    id: u32,
    icon_id: u16,
    name: String,
}

impl Item {
    pub const BASE_GAME_ITEM_NAME: u32 = 10;
    pub const DLC_ITEM_NAME: u32 = 319;
    
    pub fn try_new(param_repo: &SoloParamRepository, index: i32, id: u32) -> Option<Self> {
        let icon_id = param_repo.get::<EquipParamGoods>(id)
            .map(|goods| goods.icon_id())?;
        let name = Self::get_name(id)?;
        
        Some(Self {
            index,
            id,
            icon_id,
            name
        })
    }
    
    pub fn index(&self) -> i32 {
        self.index
    }

    pub fn id(&self) -> u32 {
        self.id
    }
    
    pub fn icon_id(&self) -> u16 {
        self.icon_id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn get_name(id: u32) -> Option<String> {
        unsafe {
            read_utf16_string(MsgRepository::get_msg(
                0, Self::BASE_GAME_ITEM_NAME, id
            )).or(read_utf16_string(MsgRepository::get_msg(
                0, Self::DLC_ITEM_NAME, id
            )))
        }
    }
}

pub unsafe fn read_utf16_string(ptr: Option<NonNull<u16>>) -> Option<String> {
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