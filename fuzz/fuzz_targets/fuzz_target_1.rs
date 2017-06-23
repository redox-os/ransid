#![no_main]
#[macro_use] extern crate libfuzzer_sys;
extern crate ransid;

fuzz_target!(|data: &[u8]| {
    let mut console = ransid::Console::new(80, 24);
    console.write(data, |_event| {});
});
