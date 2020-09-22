use crate::quickjsruntime::QuickJsRuntime;
use libquickjs_sys as q;
use std::ffi::{CStr, CString};

#[allow(dead_code)]
pub fn set_module_loader(q_js_rt: &QuickJsRuntime) {
    log::trace!("setting up module loader");

    let module_normalize: q::JSModuleNormalizeFunc = Some(js_module_normalize);
    let module_loader: q::JSModuleLoaderFunc = Some(js_module_loader);

    let opaque = std::ptr::null_mut();

    unsafe { q::JS_SetModuleLoaderFunc(q_js_rt.runtime, module_normalize, module_loader, opaque) }
}

unsafe extern "C" fn js_module_normalize(
    _ctx: *mut q::JSContext,
    module_base_name: *const ::std::os::raw::c_char,
    module_name: *const ::std::os::raw::c_char,
    _opaque: *mut ::std::os::raw::c_void,
) -> *mut ::std::os::raw::c_char {
    // todo

    let base_c = CStr::from_ptr(module_base_name);
    let base_str = base_c
        .to_str()
        .expect("could not convert module_base_name to str");
    let name_c = CStr::from_ptr(module_name);
    let name_str = name_c
        .to_str()
        .expect("could not convert module_name to str");

    log::trace!(
        "js_module_normalize called. base: {}. name: {}",
        base_str,
        name_str
    );

    let c_name = CString::new(name_str).expect("could not create CString");
    c_name.into_raw()
}

unsafe extern "C" fn js_module_loader(
    _ctx: *mut q::JSContext,
    module_name: *const ::std::os::raw::c_char,
    _opaque: *mut ::std::os::raw::c_void,
) -> *mut q::JSModuleDef {
    //todo
    let module_name_c = CStr::from_ptr(module_name);
    let res = module_name_c.to_str();

    if res.is_err() {
        panic!("failed to get module name: {}", res.err().unwrap());
    }

    log::trace!(
        "js_module_loader called: {}",
        res.expect("could not get module_name")
    );

    std::ptr::null_mut()
}

#[cfg(test)]
pub mod tests {
    use crate::esruntime::EsRuntime;
    use crate::esscript::EsScript;
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn test_module_sandbox() {
        let rt: Arc<EsRuntime> = crate::esruntime::tests::TEST_ESRT.clone();
        rt.add_to_event_queue_sync(|q_js_rt| {
            q_js_rt
                .eval_module(EsScript::new(
                    "test1.mes",
                    "export const name = 'foobar';\nconsole.log('evalling module'); this;",
                ))
                .ok()
                .expect("parse mod failed");
        });

        rt.add_to_event_queue_sync(|q_js_rt| {
            q_js_rt
                .eval_module(EsScript::new(
                    "test2.mes",
                    "import {name} from 'test1.mes';\n\nconsole.log('imported name: ' + name);",
                ))
                .ok()
                .expect("parse mod2 failed");
        });

        std::thread::sleep(Duration::from_secs(1));
    }
}
