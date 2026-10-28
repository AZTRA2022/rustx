//! The V8 handler that stands between JavaScript and a Rust closure.
//!
//! Two things happen here that the application must never have to think
//! about: a panic is caught before it can unwind into C, and an argument
//! JavaScript made up is refused before the closure ever sees it.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use cef::{CefString, ImplV8Handler, V8Handler, V8Value, WrapV8Handler, rc::Rc as _, wrap_v8_handler};

use crate::value::{NativeError, Value};

use super::NativeFn;

wrap_v8_handler! {
    struct NativeHandler {
        name: Rc<str>,
        function: Rc<NativeFn>,
    }

    impl V8Handler {
        fn execute(
            &self,
            _name: Option<&CefString>,
            _object: Option<&mut V8Value>,
            arguments: Option<&[Option<V8Value>]>,
            retval: Option<&mut Option<V8Value>>,
            exception: Option<&mut CefString>,
        ) -> ::std::os::raw::c_int {
            run(&self.name, &self.function, arguments, retval, exception)
        }
    }
}

/// A V8 function object bound to a registered native function.
///
/// The closure is held by the handler rather than looked up by name on every
/// call: this runs on the hot path, and a hash lookup per call is a cost the
/// framework has no reason to charge.
pub(super) fn handler_for(name: &str, function: Rc<NativeFn>) -> V8Handler {
    NativeHandler::new(Rc::from(name), function)
}

fn run(
    name: &str,
    function: &Rc<NativeFn>,
    arguments: Option<&[Option<V8Value>]>,
    retval: Option<&mut Option<V8Value>>,
    exception: Option<&mut CefString>,
) -> ::std::os::raw::c_int {
    // Returning zero would make V8 say the function does not exist, which is
    // never what happened here. Every path below returns 1 and speaks through
    // `exception` instead.
    const HANDLED: ::std::os::raw::c_int = 1;

    let fail = |message: String| {
        if let Some(slot) = exception {
            // `try_set` writes into the caller's buffer. Assigning a new
            // `CefString` would only replace the Rust-side wrapper, and the
            // exception would never reach JavaScript.
            slot.try_set(&message);
        }
        HANDLED
    };

    let args = match read_arguments(arguments) {
        Ok(args) => args,
        Err(error) => return fail(error.message),
    };

    // A panic crossing into C is undefined behaviour, and this frame is
    // called from C. It stops here.
    let outcome = catch_unwind(AssertUnwindSafe(|| function(&args)));

    match outcome {
        Ok(Ok(value)) => {
            if let Some(slot) = retval {
                *slot = value.into_v8();
            }
            HANDLED
        }
        Ok(Err(error)) => fail(error.message),
        Err(_) => fail(format!("internal error in '{name}'")),
    }
}

fn read_arguments(arguments: Option<&[Option<V8Value>]>) -> Result<Vec<Value>, NativeError> {
    let Some(arguments) = arguments else {
        return Ok(Vec::new());
    };
    arguments
        .iter()
        .map(|argument| match argument {
            Some(value) => Value::from_v8(value),
            None => Ok(Value::Null),
        })
        .collect()
}
