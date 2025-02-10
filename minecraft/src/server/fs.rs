use std::fs;
use std::path::Path;
use std::io;

pub fn copy_dir(src: &Path, dest: &Path) -> io::Result<u64> {
    // Ensure the destination directory exists
    if !dest.exists() {
        fs::create_dir_all(dest)?;
    }

    let mut total_size = 0;

    // Read the contents of the source directory
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if path.is_dir() {
            // If the entry is a directory, recurse
            total_size += copy_dir(&path, &dest_path)?;
        } else {
            // If it's a file, copy it
            fs::copy(&path, &dest_path)?;
            total_size += entry.metadata()?.len();
        }
    }
    
    Ok(total_size)
}
