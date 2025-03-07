#[doc = "Required features: `\"Win32_Foundation\"`"]
#[cfg(feature = "Win32_Foundation")]
#[inline]
pub unsafe fn OOBEComplete(isoobecomplete: *mut super::super::Foundation::BOOL) -> ::windows_core::Result<()> {
    ::windows_targets::link!("kernel32.dll" "system" fn OOBEComplete(isoobecomplete : *mut super::super::Foundation:: BOOL) -> super::super::Foundation:: BOOL);
    OOBEComplete(isoobecomplete).ok()
}
#[doc = "Required features: `\"Win32_Foundation\"`"]
#[cfg(feature = "Win32_Foundation")]
#[inline]
pub unsafe fn RegisterWaitUntilOOBECompleted(oobecompletedcallback: OOBE_COMPLETED_CALLBACK, callbackcontext: ::core::option::Option<*const ::core::ffi::c_void>, waithandle: *mut *mut ::core::ffi::c_void) -> ::windows_core::Result<()> {
    ::windows_targets::link!("kernel32.dll" "system" fn RegisterWaitUntilOOBECompleted(oobecompletedcallback : OOBE_COMPLETED_CALLBACK, callbackcontext : *const ::core::ffi::c_void, waithandle : *mut *mut ::core::ffi::c_void) -> super::super::Foundation:: BOOL);
    RegisterWaitUntilOOBECompleted(oobecompletedcallback, ::core::mem::transmute(callbackcontext.unwrap_or(::std::ptr::null())), waithandle).ok()
}
#[doc = "Required features: `\"Win32_Foundation\"`"]
#[cfg(feature = "Win32_Foundation")]
#[inline]
pub unsafe fn UnregisterWaitUntilOOBECompleted(waithandle: *const ::core::ffi::c_void) -> ::windows_core::Result<()> {
    ::windows_targets::link!("kernel32.dll" "system" fn UnregisterWaitUntilOOBECompleted(waithandle : *const ::core::ffi::c_void) -> super::super::Foundation:: BOOL);
    UnregisterWaitUntilOOBECompleted(waithandle).ok()
}
pub type OOBE_COMPLETED_CALLBACK = ::core::option::Option<unsafe extern "system" fn(callbackcontext: *const ::core::ffi::c_void)>;
