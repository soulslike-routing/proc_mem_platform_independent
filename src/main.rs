use process_memory::ProcessHandle;
use sysinfo::System;
use process_memory::{copy_address, TryIntoProcessHandle};
use std::fs::File;
use std::io::Read;
use std::usize;
use std::process::exit;
use patternscan::scan;
use std::io::Cursor;

fn main() {
    let pid = get_pid();
    let handle = get_proc_handle(pid);

    let filepath = format!("/proc/{}/maps", pid);
    println!("{}", filepath);

    let mut f = File::open(filepath).expect("e");
    
    let mut buffer = String::new();

    f.read_to_string(&mut buffer).expect("should work");
    // println!("{}", buffer);
    let mut keep_going = false;

    let mut valid_proc = Vec::new();
    for address_space in buffer.split("\n") {
        // println!("{}", address_space);
        if !address_space.contains("DarkSoulsRemastered.exe") && !(keep_going && (!address_space.contains("/") && !address_space.contains("["))) {
            keep_going = false;
            continue;
        }
        valid_proc.push(address_space);
        keep_going = true;
    }
    // println!("pass for {}", address_space);
    for address_space in valid_proc {
        let mut memory_range = address_space.split(" ").next().expect("First should always be there").split("-");
        let start_address = match usize::from_str_radix(memory_range.next().expect("should also be there"), 16) {
            Ok(v) => v,
            Err(e) => {
                println!("error getting start_address, {}", e);
                continue;
            }
        };
        let end_address = match usize::from_str_radix(memory_range.next().expect("shuld be here if the first one is here"), 16) {
            Ok(v) => v,
            Err(e) => { /*println!("error getting end_address, {}", e);*/continue; }
        };
        let size = end_address - start_address;
        // println!("{} with size {}",  start_address, size);

        let _bytes = match copy_address(start_address, size, &handle) {
            Ok(v) => v,
            Err(e) => { /*println!("error actually reading memory: {}", e);*/continue; }
        };
        // println!("read {} bytes", _bytes.len());

        let pattern = "48 8b 0d ? ? ? ? 0f 28 f1 48 85 c9 74 ? 48 89 7c";
        let locs = match scan(Cursor::new(_bytes), &pattern) {
            Ok(v) => v,
            Err(e) => {
                println!("error getting locations: {}", e);
                continue
            }
        };

        if locs.len() > 0 {
            println!("found occurence in {} at offset {:?}", address_space, locs[0]);
        } else {
            continue;
        }

        let ad = locs[0] + 5368713216 + 3;
        let _bytes = match copy_address(ad, 4, &handle) {
            Ok(v) => v,
            Err(e) => { exit(69); }
        };
        let addr_at_ini = i32::from_le_bytes(_bytes.try_into().expect(""));
        let target = 5368713216 + locs[0] + addr_at_ini as usize + 7;

        let offset = vec![0, 0x68, 0x68, 0x28, 0x10];

        let final_address = resolve_offsets_to_final_address(target, offset, handle);
        println!("final address: {}", final_address);

        let abc = match copy_address( final_address, 4, &handle) {
            Ok(v) => v,
            Err(e) => {exit(69);}
        };
        let x = f32::from_le_bytes(abc.try_into().expect(""));//process.read_mem::<usize>(address).unwrap();




         println!("trying to read the offset");

        // filepath = format!("/proc/{}/maps", memory_range);
        // let mut buffer = Vec::new();

        // read the whole file
        // f.read_to_end(&mut buffer).expect("Should be there");


        // println!("{:?}", buffer);
    }
}

/*pub fn read_float(address: usize, handle: ProcessHandle) -> f32 {
    let read_float = process.read_mem::<f32>(
        self.resolve_offsets(
            offsets_copy,
            &process,
        )).unwrap();
    return read_float;
}
*/
pub fn resolve_offsets_to_final_address(start:usize, offsets: Vec<usize>, process: ProcessHandle) -> usize {
    let mut ptr = start.clone();
    for (index, offset) in offsets.iter().enumerate() {
        let address = ptr + offset;
        if index + 1 < offsets.len() {
            let _bytes = match copy_address(address, 8, &process) {
                Ok(v) => v,
                Err(e) => {println!("error actually reading memory from {}: {}", address, e);exit(69);}
            };
            ptr = usize::from_le_bytes(_bytes.try_into().expect(""));//process.read_mem::<usize>(address).unwrap();
            if ptr == 0 {
                return 0;
            }
        } else {
            ptr = address;
        }
    }
    return ptr;
}

// --

fn get_pid() -> i32 {
    let mut sys = System::new_all();
    sys.refresh_all();

    let ds_proc = sys.processes_by_name("DarkSouls").into_iter().filter(|proc| proc.thread_kind() == None).next()
        .expect("Is the game running?");
    // println!("{} {}, {:?}", ds_proc.pid(), ds_proc.name(), ds_proc.memory());
    return ds_proc.pid().as_u32() as i32;
}

fn get_proc_handle(pid: i32) -> ProcessHandle {
    return pid.try_into_process_handle().unwrap();
}
