use crate::{corpus, matchers};
use core::slice;
use std::{
    ffi::{CStr, CString, c_char},
    path::PathBuf,
    ptr,
    str::FromStr,
};

/// Simple wrapper around `Vec<u8>` so that C code doesn't have to allocate
/// memory on heap and then provide a function to free it.
pub struct DDCorpBuffer(Vec<u8>);

#[unsafe(no_mangle)]
pub extern "C" fn ddcorp_buffer_copy(buf: *mut DDCorpBuffer, ptr: *const u8, len: usize) {
    unsafe {
        let slice = slice::from_raw_parts(ptr, len);

        (*buf).0.extend(slice);
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn ddcorp_buffer_write_str(buf: *mut DDCorpBuffer, str: *const c_char) {
    unsafe {
        (*buf).0.extend(CStr::from_ptr(str).to_bytes());
    };
}

pub struct DDCorpCorpus(corpus::Corpus);

#[unsafe(no_mangle)]
pub extern "C" fn ddcorp_corpus_create(path: *const c_char) -> *mut DDCorpCorpus {
    let path = unsafe { CStr::from_ptr(path) }.to_str().unwrap();

    match corpus::Corpus::new(PathBuf::from(path)) {
        Ok(mut corpus) => {
            corpus.add_matcher("json".to_string(), matchers::json);
            corpus.add_matcher("bin".to_string(), matchers::binary);

            Box::into_raw(Box::new(DDCorpCorpus(corpus)))
        }
        Err(err) => {
            println!("{}", err);

            ptr::null_mut()
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ddcorp_corpus_add_runner(
    corpus: *mut DDCorpCorpus,
    path: *const c_char,
    runner: extern "C" fn(*const u8, usize, *const c_char, *mut DDCorpBuffer),
) {
    unsafe {
        let path = CStr::from_ptr(path).to_str().unwrap();

        (*corpus).0.add_runner(
            PathBuf::from_str(path).unwrap(),
            Box::new(move |input, ext| {
                let mut buf = DDCorpBuffer(Vec::new());

                runner(
                    input.as_ptr(),
                    input.len(),
                    CString::from_str(ext).unwrap().as_ptr(),
                    &mut buf,
                );

                buf.0
            }),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ddcorp_corpus_run(corpus: *mut DDCorpCorpus) -> bool {
    match unsafe { (*corpus).0.run() } {
        Ok(all_passed) => all_passed,
        Err(err) => {
            println!("{}", err);

            false
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ddcorp_corpus_free(corpus: *mut DDCorpCorpus) {
    drop(unsafe { Box::from_raw(corpus) })
}
