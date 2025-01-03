use mbn::{encode::RecordEncoder, record_ref::RecordRef, records::RecordHeader};
use std::ffi::CStr;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::raw::c_char;
use std::path::Path;

use crate::records::{RecordData, ToRecordRef};

/// C-compatible wrapper around RecordEncoder
pub struct CRecordEncoder {
    buffer: Vec<u8>,
}

#[no_mangle]
pub extern "C" fn create_record_encoder() -> *mut CRecordEncoder {
    let c_encoder = CRecordEncoder { buffer: Vec::new() };
    Box::into_raw(Box::new(c_encoder))
}

/// Destroy the `CRecordEncoder`
#[no_mangle]
pub extern "C" fn destroy_record_encoder(encoder: *mut CRecordEncoder) {
    if encoder.is_null() {
        return;
    }

    // Safety: Convert the raw pointer back to a Box and drop it
    unsafe {
        let _ = Box::from_raw(encoder);
    }
}

#[no_mangle]
pub extern "C" fn encode_records(
    encoder: *mut CRecordEncoder,
    records: *const RecordData, // Raw pointer to data
    record_count: usize,        // Number of records
) -> i32 {
    if encoder.is_null() || records.is_null() {
        return -1; // Invalid pointers
    }

    let encoder = unsafe { &mut *encoder };
    encoder.buffer.clear(); // Clear previous data
    let mut record_encoder = RecordEncoder::new(&mut encoder.buffer);

    for i in 0..record_count {
        // records is pointer to first RecordData, this iterated the pointer to i-th RecordData
        let record = unsafe { &*records.add(i) };
        let record_ref = record.to_record_ref();

        // Attempt to encode the record
        if let Err(_) = record_encoder.encode_record(&record_ref) {
            return -2; // Encoding error
        }
    }

    0 // Success
}

#[no_mangle]
pub extern "C" fn get_encoded_data(
    encoder: *const CRecordEncoder,
    output: *mut u8,
    output_size: *mut usize,
) -> i32 {
    if encoder.is_null() || output_size.is_null() {
        return -1; // Invalid pointers
    }

    let encoder = unsafe { &*encoder };
    let buffer = &encoder.buffer;

    // Check if output pointer is provided
    if !output.is_null() {
        let output_slice = unsafe { std::slice::from_raw_parts_mut(output, buffer.len()) };
        output_slice.copy_from_slice(buffer);
    }

    // Return the size of the buffer
    unsafe {
        *output_size = buffer.len();
    }

    0 // Success
}

#[no_mangle]
pub extern "C" fn write_buffer_to_file(
    encoder: *mut CRecordEncoder,
    file_path: *const c_char,
    append: bool,
) -> i32 {
    if encoder.is_null() || file_path.is_null() {
        return -1; // Invalid pointers
    }

    // Convert the C string file path to a Rust Path
    let c_str = unsafe { CStr::from_ptr(file_path) };
    let path = match c_str.to_str() {
        Ok(s) => Path::new(s),
        Err(_) => return -2, // Invalid UTF-8 in file path
    };

    let encoder = unsafe { &mut *encoder };
    let record_encoder = RecordEncoder::new(&mut encoder.buffer);

    // Write the buffer to the file
    if let Err(_) = record_encoder.write_to_file(path, append) {
        return -4; // Failed to write to file
    }

    0 // Success
}

// #[no_mangle]
// pub extern "C" fn write_buffer_to_file(
//     encoder: *const CRecordEncoder,
//     file_path: *const c_char,
//     append: bool,
// ) -> i32 {
//     if encoder.is_null() || file_path.is_null() {
//         return -1; // Invalid pointers
//     }
//
//     // Safety: Convert raw pointers to Rust references
//     let encoder = unsafe { &*encoder };
//
//     // Convert the C string file path to a Rust Path
//     let c_str = unsafe { CStr::from_ptr(file_path) };
//     let path = match c_str.to_str() {
//         Ok(s) => Path::new(s),
//         Err(_) => return -2, // Invalid UTF-8 in file path
//     };
//
//     // Open the file
//     let mut options = OpenOptions::new();
//     options.create(true);
//
//     if append {
//         options.append(true);
//     } else {
//         options.write(true).truncate(true);
//     }
//
//     let mut file = match options.open(path) {
//         Ok(f) => f,
//         Err(_) => return -3, // Failed to open file
//     };
//
//     // Write the buffer to the file
//     if let Err(_) = file.write_all(&encoder.buffer) {
//         return -4; // Failed to write to file
//     }
//
//     0 // Success
// }

// #[no_mangle]
// pub extern "C" fn encode_records(
//     encoder: *mut CRecordEncoder,
//     records: *const u8, // Raw pointer to data
//     // record_size: usize,  // Size of each record
//     record_count: usize, // Number of records
//                          // rtype: u8,           // Record type for validation
// ) -> i32 {
//     if encoder.is_null() || records.is_null() {
//         return -1; // Invalid pointers
//     }
//
//     let encoder = unsafe { &mut *encoder };
//     encoder.buffer.clear(); // Clear previous data
//     let mut record_encoder = RecordEncoder::new(&mut encoder.buffer);
//
//     // Step 4: Initialize pointer to the first record
//     let mut current_ptr = records;
//
//     for _ in 0..record_count {
//         // Step 5: Read the length field directly from the pointer
//         let length = unsafe { *current_ptr } as usize;
//         println!("Lenght :{:?}", length);
//
//         // Calculate the record size using LENGTH_MULTIPLIER
//         let record_size = length * RecordHeader::LENGTH_MULTIPLIER;
//         println!("Record SIze :{:?}", record_size);
//
//         // Step 6: Convert the current record into a RecordRef
//         let record_bytes = unsafe { std::slice::from_raw_parts(current_ptr, record_size) };
//         let record_ref = unsafe { RecordRef::new(record_bytes) };
//
//         // Safety: Convert the raw bytes into a RecordRef
//         // let record_ref = unsafe { RecordRef::new(record_ptr) };
//
//         // Attempt to encode the record
//         if let Err(_) = record_encoder.encode_record(&record_ref) {
//             return -2; // Encoding error
//         }
//
//         // Step 7: Move the pointer to the next record
//         current_ptr = unsafe { current_ptr.add(record_size) };
//     }
//
//     0 // Success
// }
