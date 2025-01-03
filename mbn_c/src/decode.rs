use mbn::{decode::RecordDecoder, record_ref::RecordRef, records::RecordHeader};
use std::ffi::CStr;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::ptr;
use std::slice;

use crate::records::RecordData;

/// C-compatible wrapper around RecordDecoder
pub struct CRecordDecoder {
    decoder: RecordDecoder<Box<dyn Read>>,
}

/// Create a new `CRecordDecoder` with an in-memory buffer as the source.
#[no_mangle]
pub extern "C" fn create_buffer_decoder(
    source: *const u8,
    source_size: usize,
) -> *mut CRecordDecoder {
    if source.is_null() || source_size == 0 {
        return ptr::null_mut(); // Return null pointer for invalid input
    }

    // Safety: Convert the raw pointer and size to a Vec<u8>
    let source_slice = unsafe { slice::from_raw_parts(source, source_size) };
    let source_buffer = Cursor::new(source_slice.to_vec());

    let decoder = CRecordDecoder {
        decoder: RecordDecoder::new(Box::new(source_buffer)),
    };

    Box::into_raw(Box::new(decoder)) // Return a raw pointer for FFI
}

/// Create a new `CRecordDecoder` with a file as the source.
#[no_mangle]
pub extern "C" fn create_file_decoder(file_path: *const libc::c_char) -> *mut CRecordDecoder {
    if file_path.is_null() {
        return ptr::null_mut(); // Return null pointer for invalid input
    }

    // Convert C string to Rust Path
    let c_str = unsafe { CStr::from_ptr(file_path) };
    let path = match c_str.to_str() {
        Ok(path) => path,
        Err(_) => return ptr::null_mut(), // Invalid UTF-8
    };

    let decoder = match create_decoder_from_file(path) {
        Ok(f) => f,
        Err(_) => return ptr::null_mut(), // Failed to open file
    };

    let c_decoder = CRecordDecoder { decoder };

    Box::into_raw(Box::new(c_decoder)) // Return a raw pointer for FFI
}

#[no_mangle]
pub extern "C" fn decode_records(
    decoder: *mut CRecordDecoder,
    output_size: *mut usize,
) -> *mut RecordData {
    if decoder.is_null() || output_size.is_null() {
        return std::ptr::null_mut(); // Invalid pointers
    }

    let decoder = unsafe { &mut *decoder };

    // Decode records into a Vec<TestRecord>
    let records = match decoder.decoder.decode_to_owned() {
        Ok(records) => records,
        Err(_) => return std::ptr::null_mut(), // Decoding error
    };

    // Map RecordEnum to RecordData
    let records_data: Vec<RecordData> = records
        .into_iter()
        .map(|record_enum| record_enum.into())
        .collect();

    // println!("{:?}", &records);

    // Return the number of records
    unsafe {
        *output_size = records_data.len();
    }

    // Allocate memory for the records on the heap and pass ownership to the caller
    let records_box = records_data.into_boxed_slice();
    Box::into_raw(records_box) as *mut RecordData
}

/// Destroy the `CRecordDecoder`
#[no_mangle]
pub extern "C" fn destroy_record_decoder(decoder: *mut CRecordDecoder) {
    if decoder.is_null() {
        return;
    }

    unsafe {
        let _ = Box::from_raw(decoder); // Drop the decoder
    }
}

fn create_decoder_from_file(
    file_path: &str,
) -> Result<RecordDecoder<Box<dyn Read>>, std::io::Error> {
    let file = File::open(file_path)?;
    let buf_reader = BufReader::new(file);
    let boxed_reader: Box<dyn Read> = Box::new(buf_reader);
    Ok(RecordDecoder::new(boxed_reader))
}
