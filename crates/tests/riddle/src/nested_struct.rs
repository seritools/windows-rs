// Bindings generated by `windows-bindgen` 0.52.0

#![allow(
    non_snake_case,
    non_upper_case_globals,
    non_camel_case_types,
    dead_code,
    clippy::all
)]
#[repr(C)]
pub struct Inner {
    pub field_i32: i32,
}
impl ::core::marker::Copy for Inner {}
impl ::core::clone::Clone for Inner {
    fn clone(&self) -> Self {
        *self
    }
}
impl ::core::fmt::Debug for Inner {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        f.debug_struct("Inner")
            .field("field_i32", &self.field_i32)
            .finish()
    }
}
impl ::windows_core::TypeKind for Inner {
    type TypeKind = ::windows_core::CopyType;
}
impl ::windows_core::RuntimeType for Inner {
    const SIGNATURE: ::windows_core::imp::ConstBuffer =
        ::windows_core::imp::ConstBuffer::from_slice(b"struct(Test.Inner;i4)");
}
impl ::core::cmp::PartialEq for Inner {
    fn eq(&self, other: &Self) -> bool {
        self.field_i32 == other.field_i32
    }
}
impl ::core::cmp::Eq for Inner {}
impl ::core::default::Default for Inner {
    fn default() -> Self {
        unsafe { ::core::mem::zeroed() }
    }
}
#[repr(C)]
pub struct Outer {
    pub field_bool: bool,
    pub field_inner: Inner,
    pub field_usize: usize,
}
impl ::core::marker::Copy for Outer {}
impl ::core::clone::Clone for Outer {
    fn clone(&self) -> Self {
        *self
    }
}
impl ::core::fmt::Debug for Outer {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        f.debug_struct("Outer")
            .field("field_bool", &self.field_bool)
            .field("field_inner", &self.field_inner)
            .field("field_usize", &self.field_usize)
            .finish()
    }
}
impl ::windows_core::TypeKind for Outer {
    type TypeKind = ::windows_core::CopyType;
}
impl ::windows_core::RuntimeType for Outer {
    const SIGNATURE: ::windows_core::imp::ConstBuffer =
        ::windows_core::imp::ConstBuffer::from_slice(
            b"struct(Test.Outer;b1;struct(Test.Inner;i4);us)",
        );
}
impl ::core::cmp::PartialEq for Outer {
    fn eq(&self, other: &Self) -> bool {
        self.field_bool == other.field_bool
            && self.field_inner == other.field_inner
            && self.field_usize == other.field_usize
    }
}
impl ::core::cmp::Eq for Outer {}
impl ::core::default::Default for Outer {
    fn default() -> Self {
        unsafe { ::core::mem::zeroed() }
    }
}
