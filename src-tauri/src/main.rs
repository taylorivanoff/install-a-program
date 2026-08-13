// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    install_a_program_lib::apply_dev_config_from_args();
    install_a_program_lib::run()
}
