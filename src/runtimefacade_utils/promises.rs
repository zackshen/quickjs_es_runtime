use crate::facades::QuickJsRuntimeFacade;
use crate::quickjs_utils::errors;
use crate::quickjs_utils::promises::new_promise_q;
use crate::quickjs_utils::promises::PromiseRef;
use crate::quickjsrealmadapter::QuickJsRealmAdapter;
use crate::quickjsruntimeadapter::QuickJsRuntimeAdapter;
use crate::valueref::JSValueRef;
use hirofa_utils::auto_id_map::AutoIdMap;
use hirofa_utils::js_utils::JsError;
use std::cell::RefCell;
thread_local! {
    static RESOLVING_PROMISES: RefCell<AutoIdMap<PromiseRef>> = RefCell::new(AutoIdMap::new());
}

/// create a new promise with a resolver/mapper
/// the resolver will run in a helper thread and thus get a result asynchronously
/// the resulting value will then be mapped to a JSValueRef by the mapper in the EventQueue
/// the promise which was returned is then resolved with the value which is returned by the mapper
/// # Example
/// ```rust
/// use quickjs_runtime::builder::QuickJsRuntimeBuilder;
/// use quickjs_runtime::quickjs_utils::{functions, objects, primitives};
/// use quickjs_runtime::quickjs_utils;
/// use hirofa_utils::js_utils::Script;
/// use std::time::Duration;
/// use quickjs_runtime::runtimefacade_utils::promises;
/// use quickjs_runtime::quickjsruntimeadapter::QuickJsRuntimeAdapter;
/// let rt = QuickJsRuntimeBuilder::new().build();
/// rt.exe_rt_task_in_event_loop(move |q_js_rt| {
///     let q_ctx = q_js_rt.get_main_context();
///      // create rust function, please note that using new_native_function_data will be the faster option
///      let func_ref = functions::new_function_q(q_ctx, "asyncTest", move |q_ctx, _this_ref, _args| {
///               let prom = promises::new_resolving_promise(q_ctx, ||{
///                   std::thread::sleep(Duration::from_secs(1));
///                   Ok(135)
///               }, |_ctx, res|{
///                   Ok(primitives::from_i32(res))
///               });
///               prom
///      }, 1).ok().expect("could not create func");
///
///      // add func to global scope
///      let global_ref = quickjs_utils::get_global_q(q_ctx);
///      objects::set_property_q(q_ctx, &global_ref, "asyncTest", &func_ref).ok()
///             .expect("could not set prop");;
///            
/// });
/// rt.eval_sync(Script::new("test_async.es", "console.log('async test');\n
/// let p = this.asyncTest(123); \n
/// console.log('p instanceof Promise = ' + p instanceof Promise);\n
/// p.then((res) => {\n
///     console.log('p resolved to ' + res);\n
/// }).catch((err) => {\n
///     console.log('p rejected to ' + err);\n
/// });
/// ")).ok().expect("script failed");
/// // wait so promise can fullfill
/// std::thread::sleep(Duration::from_secs(2));
/// ```
pub fn new_resolving_promise<P, R, M>(
    q_ctx: &QuickJsRealmAdapter,
    producer: P,
    mapper: M,
) -> Result<JSValueRef, JsError>
where
    R: Send + 'static,
    P: FnOnce() -> Result<R, String> + Send + 'static,
    M: FnOnce(&QuickJsRealmAdapter, R) -> Result<JSValueRef, JsError> + Send + 'static,
{
    // create promise
    let promise_ref = new_promise_q(q_ctx)?;
    let return_ref = promise_ref.get_promise_obj_ref();

    // add to map and keep id
    let id = RESOLVING_PROMISES.with(|map_rc| {
        let map = &mut *map_rc.borrow_mut();
        map.insert(promise_ref)
    });

    let rti_ref =
        QuickJsRuntimeAdapter::do_with(|qjs_rt| qjs_rt.get_rti_ref().expect("invalid state"));

    let ctx_id = q_ctx.id.clone();
    // go async
    QuickJsRuntimeFacade::add_helper_task(move || {
        // in helper thread, produce result
        let produced_result = producer();
        rti_ref.add_rt_task_to_event_loop_void(move |q_js_rt| {
            if let Some(q_ctx) = q_js_rt.opt_context(ctx_id.as_str()) {
                // in q_js_rt worker thread, resolve promise
                // retrieve promise
                let prom_ref = RESOLVING_PROMISES.with(|map_rc| {
                    let map = &mut *map_rc.borrow_mut();
                    map.remove(&id)
                });

                match produced_result {
                    Ok(ok_res) => {
                        // map result to JSValueRef
                        let raw_res = mapper(q_ctx, ok_res);

                        // resolve or reject promise
                        match raw_res {
                            Ok(val_ref) => {
                                prom_ref
                                    .resolve_q(q_ctx, val_ref)
                                    .ok()
                                    .expect("prom resolution failed");
                            }
                            Err(err) => {
                                // todo use error:new_error(err.get_message)
                                let err_ref = unsafe {
                                    errors::new_error(
                                        q_ctx.context,
                                        err.get_name(),
                                        err.get_message(),
                                        err.get_stack(),
                                    )
                                }
                                .ok()
                                .expect("could not create str");
                                prom_ref
                                    .reject_q(q_ctx, err_ref)
                                    .ok()
                                    .expect("prom rejection failed");
                            }
                        }
                    }
                    Err(err) => {
                        // todo use error:new_error(err)
                        let err_ref =
                            unsafe { errors::new_error(q_ctx.context, "Error", err.as_str(), "") }
                                .ok()
                                .expect("could not create str");
                        prom_ref
                            .reject_q(q_ctx, err_ref)
                            .ok()
                            .expect("prom rejection failed");
                    }
                }
            } else {
                log::error!("resolving_promise failed, context was dropped: {}", ctx_id);
            }
        });
    });

    Ok(return_ref)
}

#[cfg(test)]

pub mod tests {
    use crate::facades::tests::init_test_rt;
    use crate::quickjs_utils;
    use crate::quickjs_utils::{functions, objects, primitives};
    use crate::runtimefacade_utils::promises;
    use crate::runtimefacade_utils::promises::RESOLVING_PROMISES;
    use hirofa_utils::js_utils::Script;
    use std::time::Duration;

    #[test]
    fn test_resolving_prom() {
        let rt = init_test_rt();

        rt.exe_rt_task_in_event_loop(move |q_js_rt| {
            // create rust function, please note that using new_native_function_data will be the faster option
            let q_ctx = q_js_rt.get_main_context();
            let func_ref = functions::new_function_q(
                q_ctx,
                "asyncTest",
                move |q_ctx, _this_ref, _args| {
                    
                    promises::new_resolving_promise(
                        q_ctx,
                        || {
                            std::thread::sleep(Duration::from_millis(5));
                            Ok(135)
                        },
                        |_q_ctx, res| Ok(primitives::from_i32(res)),
                    )
                },
                1,
            )
            .ok()
            .expect("could not create func");

            assert_eq!(1, func_ref.get_ref_count());

            // add func to global scope
            let global_ref = quickjs_utils::get_global_q(q_ctx);
            let i = global_ref.get_ref_count();
            objects::set_property_q(q_ctx, &global_ref, "asyncTest", &func_ref)
                .ok()
                .expect("could not set prop");
            assert_eq!(i, global_ref.get_ref_count());
            assert_eq!(2, func_ref.get_ref_count());
        });
        log::trace!("running gc after resolving promise init");
        //rt.gc_sync();

        rt.eval_sync(Script::new(
            "test_async.es",
            "console.log('async test');\n
         let p = this.asyncTest(123); \n
         console.log('p instanceof Promise = ' + p instanceof Promise);\n
         p.then((res) => {\n
             console.log('p resolved to ' + res);\n
         }).catch((err) => {\n
             console.log('p rejected to ' + err);\n
         });
         ",
        ))
        .ok()
        .expect("script failed");
        rt.gc_sync();
        // wait so promise can fullfill
        std::thread::sleep(Duration::from_secs(10));
        assert!(RESOLVING_PROMISES.with(|rc| { (*rc.borrow()).is_empty() }))
    }

    #[test]
    fn test_simple_prom() {
        let rt = init_test_rt();

        // todo test with context_init_hooks disabled

        rt.exe_rt_task_in_event_loop(|q_js_rt| {

            let q_ctx = q_js_rt.get_main_context();
             q_ctx.eval(Script::new(
                "test_simple_prom.es",
                "this.test = function(){return new Promise((resolve, reject) => {resolve('abc');}).then((a) => {return(a.toUpperCase());})}",
            )).ok().expect("p1");

            q_js_rt.run_pending_jobs_if_any();

            let global = quickjs_utils::get_global_q(q_ctx);
            let e_res = functions::invoke_member_function_q(q_ctx, &global,  "test", vec![quickjs_utils::new_null_ref()]);
            if e_res.is_err() {
                panic!("{}", e_res.err().unwrap());
            }
            let _p_ref = e_res.ok().unwrap();

            q_js_rt.run_pending_jobs_if_any();

        });

        std::thread::sleep(Duration::from_secs(1));
    }
}
