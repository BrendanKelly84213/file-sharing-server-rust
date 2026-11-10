use crate::filesystem::datastructures::FEntry;
use std::fs::File;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static INSTANCE_INITIALIZED: AtomicBool = AtomicBool::new(false);

#[allow(dead_code)]
pub struct FileSystemManager {
    max_files: i32,
    max_blocks: i32,
    disk: Option<File>, // the file that stores the simulated file system
    global_lock: Mutex<()>,
    inode_table: Vec<FEntry>,   // Array of inodes
    free_block_list: Vec<bool>, // Bitmap for free blocks
}

#[allow(dead_code)]
impl FileSystemManager {
    const BLOCK_SIZE: i32 = 128; // Example block size

    pub fn new(filename: &str, total_size: usize) -> Result<Self, String> {
        // Initialize the file system manager with a file
        if !INSTANCE_INITIALIZED.swap(true, Ordering::SeqCst) {
            // TODO Initialize the file system
            let _ = (filename, total_size);
            Ok(Self {
                max_files: 5,
                max_blocks: 10,
                disk: None,
                global_lock: Mutex::new(()),
                inode_table: Vec::new(),
                free_block_list: Vec::new(),
            })
        } else {
            Err("FileSystemManager is already initialized.".to_string())
        }
    }

    pub fn create_file(&self, _file_name: &str) -> Result<(), String> {
        // TODO
        Err("Method not implemented yet.".to_string())
    }

    // TODO: Add read_file, write_file and other required methods,
}
