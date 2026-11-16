#[repr(C)]
pub struct CSKeyAssign {
    pub _pad: [usize; 2],
    pub input_mapper: *const InputMapper,
}

impl CSKeyAssign {
    pub fn input_mapper(&self) -> Option<&InputMapper> {
        unsafe { self.input_mapper.as_ref() }
    }
}

#[repr(C)]
pub struct InputMapper {
    pub _pad: usize,
    pub inner: *const InputMapperInner
}

impl InputMapper {
    pub fn inner(&self) -> Option<&InputMapperInner> {
        unsafe { self.inner.as_ref() }
    }
}

#[repr(C)]
pub struct InputMapperInner {
    pub _pad: [usize; 2],
    pub begin: *const Begin,
}

impl InputMapperInner {
    pub fn begin(&self) -> Option<&Begin> {
        unsafe { self.begin.as_ref() }
    }
}

#[repr(C)]
pub struct Begin {
    pub binding_vectors: *const BindingVectors,
}

impl Begin {
    pub fn binding_vectors(&self) -> Option<&BindingVectors> {
        unsafe { self.binding_vectors.as_ref() }
    }
}

#[repr(C)]
pub struct BindingVectors {
    pub _pad1: [usize; 7],
    pub digital: BindingVector,
    pub _pad2: [usize; 6],
    pub analog: BindingVector,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BindingVector {
    pub begin: *const usize,
    pub end: *const usize,
}