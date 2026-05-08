use eldenring::cs::CSCamera;
use fromsoftware_shared::{F32ViewMatrix, FromSingleton};
use crate::pointers::jump_pointers;

pub struct FD4PadManager;

impl FromSingleton for FD4PadManager {}

impl FD4PadManager {
    const PRESSES_OFFSETS: &'static [usize] = &[0x18, 0x8, 0x7C8, 0x10];

    pub fn presses(&self) -> Option<&[i32; 8]> {
        unsafe {
            let ptr = jump_pointers::<[i32; 8]>(&self as *const &Self as *const usize, Self::PRESSES_OFFSETS);
            ptr.as_ref()
        }
    }

    pub fn check_near_deref<T>(&self, elements: &[T]) -> isize where T: PartialEq {
        unsafe {
            let ptr = jump_pointers::<u8>(&self as *const &Self as *const usize, &[0x18]);
            if ptr.is_null() {
                return -2;
            }
            let bytes = std::slice::from_raw_parts(elements as *const [T] as *const u8, elements.len() * size_of::<T>());
            'search: for offset in 0..80_000 {
                for (i, elem) in bytes.iter().enumerate() {
                    if *ptr.offset(offset + i as isize) != *elem {
                        continue 'search;
                    }
                }
                return offset;
            }
            -1
        }
    }
}

pub fn reset_camera(camera: &mut F32ViewMatrix, stored: &F32ViewMatrix) {
    camera.0 = stored.0;
    camera.1 = stored.1;
    camera.2 = stored.2;
}

pub fn try_reset_camera(csc: Option<&mut CSCamera>, matrix: Option<&F32ViewMatrix>) {
    match (csc, matrix) {
        (Some(csc), Some(matrix)) => {
            reset_camera(&mut csc.pers_cam_1.matrix, matrix);
            reset_camera(&mut csc.pers_cam_2.matrix, matrix);
        },
        _ => {}
    }
}

pub fn get_view_matrix(csc: Option<&mut CSCamera>) -> Option<F32ViewMatrix> {
    csc.map(|csc| csc.pers_cam_1.matrix)
}