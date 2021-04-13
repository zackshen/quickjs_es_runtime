//! Utils for working with Date objects

use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjs_utils::{functions, objects, primitives};
use crate::quickjscontext::QuickJsContext;
use crate::valueref::JSValueRef;
use libquickjs_sys as q;

/// create a new instance of a Date object
pub fn new_date_q(context: &QuickJsContext) -> Result<JSValueRef, EsError> {
    unsafe { new_date(context.context) }
}

/// create a new instance of a Date object
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn new_date(context: *mut q::JSContext) -> Result<JSValueRef, EsError> {
    let constructor = quickjs_utils::get_constructor(context, "Date")?;
    let date_ref = functions::call_constructor(context, &constructor, &[])?;
    Ok(date_ref)
}

/// check if a JSValueRef is an instance of Date
pub fn is_date_q(context: &QuickJsContext, obj_ref: &JSValueRef) -> bool {
    unsafe { is_date(context.context, obj_ref) }
}

/// check if a JSValueRef is an instance of Date
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn is_date(context: *mut q::JSContext, obj_ref: &JSValueRef) -> bool {
    objects::is_instance_of_by_name(context, obj_ref, "Date").unwrap_or(false)
}

/// set the timestamp for a Date object
pub fn set_time_q(
    context: &QuickJsContext,
    date_ref: &JSValueRef,
    timestamp: f64,
) -> Result<(), EsError> {
    unsafe { set_time(context.context, date_ref, timestamp) }
}

/// set the timestamp for a Date object
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn set_time(
    context: *mut q::JSContext,
    date_ref: &JSValueRef,
    timestamp: f64,
) -> Result<(), EsError> {
    functions::invoke_member_function(
        context,
        date_ref,
        "setTime",
        vec![primitives::from_f64(timestamp)],
    )?;
    Ok(())
}
/// get the timestamp from a Date object
pub fn get_time_q(context: &QuickJsContext, date_ref: &JSValueRef) -> Result<f64, EsError> {
    unsafe { get_time(context.context, date_ref) }
}
/// get the timestamp from a Date object
/// # Safety
/// When passing a context pointer please make sure the corresponding QuickJsContext is still valid
pub unsafe fn get_time(context: *mut q::JSContext, date_ref: &JSValueRef) -> Result<f64, EsError> {
    let time_ref = functions::invoke_member_function(context, date_ref, "getTime", vec![])?;
    if time_ref.is_f64() {
        primitives::to_f64(&time_ref)
    } else {
        primitives::to_i32(&time_ref).map(|i| i as f64)
    }
}

#[cfg(test)]
pub mod tests {

    use crate::esruntime::tests::init_test_rt;
    use crate::esruntime::EsRuntime;
    use crate::quickjs_utils::dates;
    use crate::quickjs_utils::dates::{get_time_q, is_date_q, set_time_q};
    use std::sync::Arc;

    #[test]
    fn test_date() {
        let rt: Arc<EsRuntime> = init_test_rt();
        rt.exe_rt_task(|q_js_rt| {
            let q_ctx = q_js_rt.get_main_context();
            let date_ref = dates::new_date_q(q_ctx).ok().expect("new_date failed");
            assert!(is_date_q(q_ctx, &date_ref));

            set_time_q(q_ctx, &date_ref, 2147483648f64)
                .ok()
                .expect("could not set time");
            let gt_res = get_time_q(q_ctx, &date_ref);
            match gt_res {
                Ok(t) => {
                    assert_eq!(t, 2147483648f64);
                }
                Err(e) => {
                    panic!("get time failed: {}", e);
                }
            }

            set_time_q(q_ctx, &date_ref, 2f64)
                .ok()
                .expect("could not set time");
            let gt_res = get_time_q(q_ctx, &date_ref);
            match gt_res {
                Ok(t) => {
                    assert_eq!(t, 2f64);
                }
                Err(e) => {
                    panic!("get time 2 failed: {}", e);
                }
            }
        });
    }
}
