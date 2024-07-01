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

    let mut f = match File::open(filepath) {
        Ok(v) => v,
        Err(e) => {println!("error opening file at /proc/{}/maps: {}", pid, e);exit(69);}
    };
    
    let mut entire_proc_file_buffer = String::new();

    match f.read_to_string(&mut entire_proc_file_buffer) {
        Ok(bytes_read_count) => bytes_read_count,
        Err(e) => {println!("error reading file: {}", e);exit(69);}
    };

    let available_address_spaces = get_available_address_spaces(&entire_proc_file_buffer);

    let mut base_address: Option<usize> = None;

    for memory_region in available_address_spaces {
        let mut memory_range = match memory_region.split(" ").next(){
            Some(v) => v.split("-"),
            None => {
                println!("Unable to extract memory range from memory region: {}", memory_region);
                exit(69);
            }
        };
        let start_address = match usize::from_str_radix(
            match memory_range.next() {
                Some(v) => v,
                None => {
                    println!("No candidates available for start address in {:?}", memory_range);
                    exit(69);
                }
            },
            16
        ) {
            Ok(v) => v,
            Err(e) => {
                println!("error converting string of start_address to usize, {}", e);
                continue;
            }
        };

        let end_address = match usize::from_str_radix(
            match memory_range.next() {
                Some(v) => v,
                None => {
                    println!("No candidates available for end address in {:?}", memory_range);
                    exit(69);
                }
            },
            16
        ) {
            Ok(v) => v,
            Err(e) => {
                println!("error converting string of end_address to usize, {}", e);
                continue;
            }
        };
        if base_address == None {
            base_address = Some(end_address);
        }
        let size = end_address - start_address;

        let entire_memory_region: Vec<u8> = match copy_address(start_address, size, &handle) {
            Ok(v) => v,
            Err(e) => {
                println!("Error reading memory region {:?} : {}", memory_range, e);
                continue;
            }
        };

        let player_pos_pattern = "48 8b 0d ? ? ? ? 0f 28 f1 48 85 c9 74 ? 48 89 7c";
        let locs = match scan(Cursor::new(entire_memory_region), &player_pos_pattern) {
            Ok(v) => v,
            Err(e) => {
                println!("Error scanning for pattern for player_pos_location: {}", e);
                continue
            }
        };

        if locs.len() == 0 {
            continue;
        }

        let initial_search = locs[0];
        // 5368713216
        let ad = initial_search + base_address.expect("") + 3; // 3 is address_offset from player_pos
        let _bytes = match copy_address(ad, 4, &handle) {
            Ok(v) => v,
            Err(_e) => { exit(69); }
        };
        let addr_at_ini = i32::from_le_bytes(_bytes.try_into().expect(""));
        let target = base_address.expect("") + locs[0] + addr_at_ini as usize + 7;

        let offset = vec![0, 0x68, 0x68, 0x28, 0x10];

        let final_address = resolve_offsets_to_final_address(target, offset, handle);
        println!("final address: {}", final_address);

        let abc = match copy_address( final_address, 4, &handle) {
            Ok(v) => v,
            Err(_e) => {exit(69);}
        };
        let x = f32::from_le_bytes(abc.try_into().expect(""));//process.read_mem::<usize>(address).unwrap();




         println!("player x: {}", x);
    }
}

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

fn get_available_address_spaces(all_spaces: &String) -> Vec<&str>{
    let mut keep_going = false;

    let mut valid_adr_spc = Vec::new();
    for address_space in all_spaces.split("\n") {
        // println!("{}", address_space);
        if !address_space.contains("DarkSoulsRemastered.exe") && !(keep_going && (!address_space.contains("/") && !address_space.contains("["))) {
            keep_going = false;
            continue;
        }
        valid_adr_spc.push(address_space);
        keep_going = true;
    }
    return valid_adr_spc;
}
