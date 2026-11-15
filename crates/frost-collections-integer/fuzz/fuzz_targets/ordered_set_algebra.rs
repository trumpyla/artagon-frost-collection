#![no_main]

use frost_fuzz::check_ordered_set_algebra;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    check_ordered_set_algebra(data);
});
