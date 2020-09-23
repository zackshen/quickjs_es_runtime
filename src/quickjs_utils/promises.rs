use crate::eserror::EsError;
use crate::quickjs_utils;
use crate::quickjs_utils::functions;
use crate::quickjs_utils::objects::is_instance_of_by_name;
use crate::quickjsruntime::QuickJsRuntime;
use crate::valueref::JSValueRef;
use libquickjs_sys as q;

#[allow(dead_code)]
pub fn is_promise(q_js_rt: &QuickJsRuntime, obj_ref: &JSValueRef) -> Result<bool, EsError> {
    is_instance_of_by_name(q_js_rt, obj_ref, "Promise")
}

pub struct PromiseRef {
    promise_obj_ref: JSValueRef,
    reject_function_obj_ref: JSValueRef,
    resolve_function_obj_ref: JSValueRef,
}
#[allow(dead_code)]
impl PromiseRef {
    fn get_promise_obj_ref(&self) -> JSValueRef {
        self.promise_obj_ref.clone()
    }

    fn resolve(&self, q_js_rt: &QuickJsRuntime, value: JSValueRef) -> Result<(), EsError> {
        crate::quickjs_utils::functions::call_function(
            q_js_rt,
            &self.resolve_function_obj_ref,
            &[value],
            None,
        )?;
        Ok(())
    }
    fn reject(&self, q_js_rt: &QuickJsRuntime, value: JSValueRef) -> Result<(), EsError> {
        crate::quickjs_utils::functions::call_function(
            q_js_rt,
            &self.reject_function_obj_ref,
            &[value],
            None,
        )?;
        Ok(())
    }
}

#[allow(dead_code)]
pub fn new_promise(q_js_rt: &QuickJsRuntime) -> Result<PromiseRef, EsError> {
    let mut promise_resolution_functions = [quickjs_utils::new_null(), quickjs_utils::new_null()];

    let prom_val = unsafe {
        q::JS_NewPromiseCapability(q_js_rt.context, promise_resolution_functions.as_mut_ptr())
    };

    let resolve_func_val = *promise_resolution_functions.get(0).unwrap();
    let reject_func_val = *promise_resolution_functions.get(1).unwrap();

    let resolve_function_obj_ref = JSValueRef::new_no_free(resolve_func_val);
    let reject_function_obj_ref = JSValueRef::new_no_free(reject_func_val);
    assert!(functions::is_function(q_js_rt, &resolve_function_obj_ref));
    assert!(functions::is_function(q_js_rt, &reject_function_obj_ref));

    let promise_obj_ref = JSValueRef::new(prom_val);

    Ok(PromiseRef {
        promise_obj_ref,
        reject_function_obj_ref,
        resolve_function_obj_ref,
    })
}

pub(crate) fn init_promise_rejection_tracker(q_js_rt: &QuickJsRuntime) {
    let tracker: q::JSHostPromiseRejectionTracker = Some(promise_rejection_tracker);

    unsafe {
        q::JS_SetHostPromiseRejectionTracker(q_js_rt.runtime, tracker, std::ptr::null_mut());
    }
}

#[allow(dead_code)]
pub fn add_promise_reactions(
    q_js_rt: &QuickJsRuntime,
    promise_obj_ref: &JSValueRef,
    then_func_obj_ref_opt: Option<JSValueRef>,
    catch_func_obj_ref_opt: Option<JSValueRef>,
    finally_func_obj_ref_opt: Option<JSValueRef>,
) -> Result<(), EsError> {
    assert!(is_promise(q_js_rt, promise_obj_ref)?);

    if let Some(then_func_obj_ref) = then_func_obj_ref_opt {
        functions::invoke_member_function(q_js_rt, &promise_obj_ref, "then", &[then_func_obj_ref])?;
    }
    if let Some(catch_func_obj_ref) = catch_func_obj_ref_opt {
        functions::invoke_member_function(
            q_js_rt,
            &promise_obj_ref,
            "catch",
            &[catch_func_obj_ref],
        )?;
    }
    if let Some(finally_func_obj_ref) = finally_func_obj_ref_opt {
        functions::invoke_member_function(
            q_js_rt,
            &promise_obj_ref,
            "finally",
            &[finally_func_obj_ref],
        )?;
    }

    Ok(())
}

unsafe extern "C" fn promise_rejection_tracker(
    _ctx: *mut q::JSContext,
    _promise: q::JSValue,
    _reason: q::JSValue,
    is_handled: ::std::os::raw::c_int,
    _opaque: *mut ::std::os::raw::c_void,
) {
    if is_handled == 0 {
        log::error!("unhandled promise rejection detected");
    }
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use crate::quickjs_utils::promises::{add_promise_reactions, is_promise, new_promise};
    use crate::quickjs_utils::{functions, new_null_ref, primitives};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_instance_of_prom() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        let io = rt.add_to_event_queue_sync(|q_js_rt| {
            let res = q_js_rt.eval(EsScript::new(
                "test_instance_of_prom.es",
                "(new Promise((res, rej) => {}));",
            ));
            match res {
                Ok(v) => is_promise(q_js_rt, &v)
                    .ok()
                    .expect("could not get instanceof"),
                Err(e) => {
                    panic!("err: {}", e);
                }
            }
        });
        assert!(io);
    }

    #[test]
    fn new_prom() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            let func_ref = q_js_rt
                .eval(EsScript::new(
                    "new_prom.es",
                    "(function(p){p.then((res) => {console.log('prom resolved to ' + res);});});",
                ))
                .ok()
                .unwrap();

            let prom = new_promise(q_js_rt).ok().unwrap();

            functions::call_function(q_js_rt, &func_ref, &[prom.get_promise_obj_ref()], None)
                .ok()
                .unwrap();

            prom.resolve(q_js_rt, primitives::from_i32(743))
                .ok()
                .expect("resolve failed");
        });
        std::thread::sleep(Duration::from_secs(1));
    }

    #[test]
    fn new_prom2() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            let func_ref = q_js_rt
                .eval(EsScript::new(
                    "new_prom.es",
                    "(function(p){p.catch((res) => {console.log('prom rejected to ' + res);});});",
                ))
                .ok()
                .unwrap();

            let prom = new_promise(q_js_rt).ok().unwrap();

            functions::call_function(q_js_rt, &func_ref, &vec![prom.get_promise_obj_ref()], None)
                .ok()
                .unwrap();

            prom.reject(q_js_rt, primitives::from_i32(130))
                .ok()
                .expect("reject failed");
        });
        std::thread::sleep(Duration::from_secs(1));
    }

    #[test]
    fn test_promise_reactions() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            let prom_ref = q_js_rt
                .eval(EsScript::new(
                    "test_promise_reactions.es",
                    "(new Promise(function(resolve, reject) {resolve(364);}));",
                ))
                .ok()
                .expect("script failed");

            let then_cb = functions::new_function(
                q_js_rt,
                "testThen",
                |_this, args| {
                    let res = primitives::to_i32(args.get(0).unwrap()).ok().unwrap();
                    log::trace!("prom resolved with: {}", res);
                    Ok(new_null_ref())
                },
                1,
            )
            .ok()
            .expect("could not create cb");
            let finally_cb = functions::new_function(
                q_js_rt,
                "testThen",
                |_this, _args| {
                    log::trace!("prom finalized");
                    Ok(new_null_ref())
                },
                1,
            )
            .ok()
            .expect("could not create cb");

            add_promise_reactions(q_js_rt, &prom_ref, Some(then_cb), None, Some(finally_cb))
                .ok()
                .expect("could not add promise reactions");
        });
        std::thread::sleep(Duration::from_secs(1));
    }
}
