use sysinfo::System;

fn main() {
    let mut sys = System::new_all();
    sys.refresh_all();

    let ds_proc = sys.processes_by_name("DarkSouls").into_iter().filter(|proc| proc.thread_kind() == None).next()
        .expect("Is the game running?");
    println!("{} {}, {:?} {:?}", ds_proc.pid(), ds_proc.name(), ds_proc.memory());
}
